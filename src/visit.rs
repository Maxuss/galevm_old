use crate::structs::{StructureInstance, StructureTemplate};
use crate::tks::{Literal, Token, TokenChain};
use crate::var::{ContainingScope, ScopedValue};
use crate::ToResult;
use anyhow::bail;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use colored::Colorize;
use rand::RngCore;
use crate::features::StdFeature;
use crate::fns::{EXTERN_FNS, StaticFnType};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Scope {
    Struct,
    StaticFunction,
    InstanceFunction,
    Global,
}

pub trait TokenProvider {
    fn next_token(&mut self) -> anyhow::Result<Token>;
    fn peek_token(&mut self) -> anyhow::Result<Token>;
    fn add_token(&mut self, tk: Token);
    fn insert_token(&mut self, tk: Token, at: usize);
}

pub trait ScopeProvider {
    fn add_std_feature(&mut self, feature: StdFeature);

    fn get_type_ptr(&self, typename: String) -> anyhow::Result<usize>;

    fn resolve_var(&self, name: &str) -> anyhow::Result<Literal>;
    fn resolve_const(&self, name: &str) -> anyhow::Result<Literal>;

    fn import(&mut self, from: String, name: String);
    fn export(&mut self, name: String);

    fn add_var(&mut self, name: String, var: Literal);
    fn add_const(&mut self, name: String, var: Literal);

    fn add_static_fn(
        &mut self,
        name: String,
        output_ty: String,
        param_names: Vec<String>,
        tks: TokenChain,
    );
    fn add_inst_fn(
        &mut self,
        name: String,
        output_ty: String,
        param_names: Vec<String>,
        tks: TokenChain,
    );
    fn add_extern_fn(
        &mut self,
        name: String,
        output_ty: String,
        param_names: Vec<String>,
        ptr: usize
    );

    fn register_type(&mut self, structure: &StructureTemplate);
    fn resolve_type(&mut self, name: String) -> Arc<Mutex<StructureTemplate>>;
    fn resolve_type_raw(&mut self, ptr: usize) -> StructureTemplate;

    fn current_struct_name(&self) -> Option<String>;
    fn add_struct_name(&mut self, name: String);

    fn call_inst_fn(&mut self, name: String, this: Box<StructureInstance>, params: TokenChain) -> Literal;
    fn call_static_fn(&mut self, name: String, params: TokenChain) -> Literal;
    fn call_ptr_fn(&mut self, ptr: usize, params: TokenChain) -> Literal;

    fn resolve_any_var(&self, name: &str) -> Literal {
        let var = self.resolve_var(name);
        if var.is_ok() {
            var.unwrap().to_owned()
        } else {
            self.resolve_const(name).unwrap().to_owned()
        }
    }
}

pub trait GlobalScope {
    fn push_scope_level(&mut self, scope: Scope);
    fn pop_scope_level(&mut self) -> Scope;
    fn scope_level(&mut self) -> Scope;
    fn push_scope(&mut self, name: String, scope: ContainingScope);
}

pub trait LiteralStack {
    fn push_stack(&mut self, value: Literal);
    fn pop_stack(&mut self) -> Literal;

    fn move_scope(&mut self, name: String);
    fn scope_name(&self) -> String;
    fn drop_scope(&mut self, name: String) -> Arc<Mutex<ContainingScope>>;

    fn get_scope(&self, name: String) -> &Arc<Mutex<ContainingScope>>;
}

pub trait Visitor: TokenProvider + Clone + ScopeProvider + GlobalScope + LiteralStack {
    fn visit<V>(&mut self, visitable: &mut V)
    where
        V: Visitable;

    fn process(&mut self);

    fn process_until(&mut self, until: usize);
    fn process_between(&mut self, from: usize, to: usize);

    fn load_chain(&mut self, chain: &mut TokenChain) {
        let iter = chain.iter();
        for ele in iter {
            self.add_token(ele.to_owned());
        }
    }

    fn load_chain_rev(&mut self, chain: &mut TokenChain) {
        let iter = chain.iter();
        for ele in iter.rev() {
            self.add_token(ele.to_owned());
        }
    }

    fn process_chain(&mut self, chain: &mut TokenChain) {
        for ele in chain {
            self.visit(ele);
        }
    }
}

pub trait Visitable {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()>
    where
        V: Visitor;
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Vm {
    free: usize,
    pos: usize,
    tks: VecDeque<Token>,
    lit_stack: Vec<Literal>,
    current_scope: String,
    scopes: HashMap<String, Arc<Mutex<ContainingScope>>>,
    struct_names: VecDeque<String>,
    scope_types: VecDeque<Scope>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            free: 0,
            pos: 0,
            tks: VecDeque::new(),
            lit_stack: vec![],
            current_scope: "global".to_string(),
            scopes: HashMap::from([(
                "global".to_string(),
                Arc::new(Mutex::new(ContainingScope::new())),
            )]),
            struct_names: Default::default(),
            scope_types: VecDeque::from(vec![Scope::Global]),
        }
    }

    pub fn emit_error(&self, message: &str) -> ! {
        println!("{} {}", "[Error]".red(), message.bright_red());
        panic!("Failure")
    }

    pub fn merged_scope(&self) -> Arc<Mutex<ContainingScope>> {
        let current = self.scopes.get(&self.current_scope).unwrap().clone();
        let imports = current.lock().unwrap().imports().clone();
        for (scope, values) in imports {
            let scope = self.scopes.get(&scope).unwrap().clone();
            for name in values {
                let value = scope.lock().unwrap().get_any_value(&name.clone());
                match value {
                    None => {
                        panic!("Tried to import non-existent value {:?}!", name)
                    }
                    Some(scoped) => match scoped {
                        ScopedValue::Constant(v) => current.lock().unwrap().add_const(&name, v),
                        ScopedValue::Mutable(v) => current.lock().unwrap().add_var(&name, v),
                        ScopedValue::Type(v) => current.lock().unwrap().add_struct(&name, v.lock().unwrap().to_owned()),
                        ScopedValue::StaticFn(v) => {
                            match v {
                                StaticFnType::Standard(std) => current.lock().unwrap().add_prebuilt_static_fn(&name, std),
                                StaticFnType::Extern(ext) => current.lock().unwrap().add_prebuilt_extern_fn(&name, ext)
                            }
                        },
                    },
                }
            }
        }
        current
    }
}

impl TokenProvider for Vm {
    fn next_token(&mut self) -> anyhow::Result<Token> {
        Ok(self.tks.pop_back().unwrap())
    }

    fn peek_token(&mut self) -> anyhow::Result<Token> {
        let tks = self.tks.clone();
        let mut iter = tks.iter().rev().peekable();
        let peek = iter.peek();
        if peek.is_some() {
            Ok(peek.unwrap().to_owned().to_owned())
        } else {
            bail!("No tokens provided!")
        }
    }

    fn add_token(&mut self, tk: Token) {
        self.tks.push_front(tk)
    }

    fn insert_token(&mut self, tk: Token, at: usize) {
        self.tks.insert(at, tk);
    }
}

impl ScopeProvider for Vm {
    fn add_std_feature(&mut self, feature: StdFeature) {
        feature.include(self)
    }

    fn get_type_ptr(&self, typename: String) -> anyhow::Result<usize> {
        let k = self.merged_scope().lock().unwrap().get_struct_ptr(typename.clone()).expect(&format!("Could not find structure {} in current scope!", typename));
        Ok(u64::from_str_radix(k.trim_start_matches("0x"), 16).unwrap() as usize)
    }

    fn resolve_var(&self, name: &str) -> anyhow::Result<Literal> {
        let value = self
            .scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .get_var(name);
        value.to_result()
    }

    fn resolve_const(&self, name: &str) -> anyhow::Result<Literal> {
        self.merged_scope()
            .lock()
            .unwrap()
            .get_const(name)
            .to_result()
    }

    fn import(&mut self, from: String, name: String) {
        self.scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .import(&from, &name);
    }

    fn export(&mut self, name: String) {
        self.scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .export(&name);
    }


    fn add_var(&mut self, name: String, var: Literal) {
        self.scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .add_var(&name, var);
    }

    fn add_const(&mut self, name: String, var: Literal) {
        self.scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .add_const(&name, var)
    }

    fn add_static_fn(
        &mut self,
        name: String,
        output_ty: String,
        param_names: Vec<String>,
        tks: TokenChain,
    ) {
        self.scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .add_static_fn(&name, output_ty, param_names, tks);
    }

    fn add_inst_fn(&mut self, name: String, output_ty: String, param_names: Vec<String>, tks: TokenChain) {
        if *self.scope_types.front().unwrap() != Scope::Struct {
            panic!("Can not add instance function outside of structure!")
        }
        let struct_name = self.current_struct_name().unwrap();
        let str = self.get_scope("global".to_string()).lock().unwrap().get_struct(&struct_name).unwrap();
        let mut str = str.lock().unwrap();
        str.add_inst_fn(name, output_ty, param_names, tks);
    }

    fn add_extern_fn(&mut self, name: String, output_ty: String, param_names: Vec<String>, ptr: usize) {
        self.scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .add_extern_fn(&name, output_ty, param_names, ptr);
    }

    fn register_type(&mut self, structure: &StructureTemplate) {
        self.scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .add_struct(&format!("0x{:2x}", rand::thread_rng().next_u64()), structure.to_owned());
    }

    fn resolve_type(&mut self, name: String) -> Arc<Mutex<StructureTemplate>> {
        self.merged_scope()
            .lock()
            .unwrap()
            .get_struct(&name)
            .expect(format!("Could not find type {:?}!", name).as_str())
    }

    fn resolve_type_raw(&mut self, ptr: usize) -> StructureTemplate {
        self.merged_scope().lock().unwrap().get_struct_raw(ptr).expect(format!("Could not find type by pointer 0x{:2x}!", ptr).as_str())
    }

    fn current_struct_name(&self) -> Option<String> {
        self.struct_names.iter().peekable().peek().map(|it| it.to_string())
    }

    fn add_struct_name(&mut self, name: String) {
        self.struct_names.push_front(name);
    }

    fn call_inst_fn(&mut self, name: String, this: Box<StructureInstance>, params: TokenChain) -> Literal {
        if self.scope_level() == Scope::Struct {
            self.emit_error("Can not call functions inside a raw struct scope!")
        }
        let mut params = params.clone();
        let params = params
            .iter_mut()
            .map(|it| it.as_lit_advanced(self, "Expected a literal-like!"))
            .collect();
        let str = self.merged_scope().lock().unwrap().get_struct(&this.typename()).unwrap();
        let mut str = str.lock().unwrap();
        str.call_inst_fn(*this, name, params, self)
    }

    fn call_static_fn(&mut self, name: String, params: TokenChain) -> Literal {
        if self.scope_level() == Scope::Struct {
            self.emit_error("Can not call functions inside a raw struct scope!")
        }

        let mut params = params.clone();
        let params = params
            .iter_mut()
            .map(|it| it.as_lit_advanced(self, "Expected a literal-like!"))
            .collect();
        if name.contains(".") {
            let (str, name) = name.rsplit_once(".").unwrap();
            let str = if str.contains("::") {
                let (scope, str) = str.rsplit_once("::").unwrap();
                self.get_scope(scope.to_string()).lock().unwrap().get_struct(str).unwrap()
            } else {
                self.merged_scope().lock().unwrap().get_struct(str).unwrap()
            };
            let mut str = str.lock().unwrap();
            str.call_static_fn(name.to_string(), params, self)
        } else if name.contains("::") {
            let (scope, fnc_name) = name.rsplit_once("::").unwrap();
            let fnc = self
                .get_scope(scope.to_owned())
                .lock()
                .unwrap()
                .get_static_fn(&fnc_name)
                .unwrap();
            fnc.call(params, Some(self))
        } else {
            let fnc = self
                .merged_scope()
                .lock()
                .unwrap()
                .get_static_fn(&name)
                .expect(&format!(
                    "Could not find function {} in current scope!",
                    name
                ));
            fnc.call(params, Some(self))
        }
    }

    fn call_ptr_fn(&mut self, ptr: usize, params: TokenChain) -> Literal {
        if self.scope_level() == Scope::Struct {
            self.emit_error("Can not call functions inside a raw struct scope!")
        }
        let fns = EXTERN_FNS.lock().unwrap();
        if fns.len() < ptr {
            panic!("Tried to call an nonexistent ptr-bound external function: 0x{:2x}", ptr)
        };
        let mut params = params.clone();
        let params = params
            .iter_mut()
            .map(|it| it.as_lit_advanced(self, "Expected a literal-like!"))
            .collect();
        let fnc = &fns[ptr];
        fnc.call((params, ))
    }
}

impl GlobalScope for Vm {
    fn push_scope_level(&mut self, scope: Scope) {
        self.scope_types.push_front(scope);
    }

    fn pop_scope_level(&mut self) -> Scope {
        self.scope_types.pop_front().unwrap()
    }

    fn scope_level(&mut self) -> Scope {
        *self.scope_types.front().unwrap()
    }

    fn push_scope(&mut self, name: String, scope: ContainingScope) {
        self.scopes.insert(name, Arc::new(Mutex::new(scope)));
    }

}

impl LiteralStack for Vm {
    fn push_stack(&mut self, value: Literal) {
        self.lit_stack.push(value);
    }

    fn pop_stack(&mut self) -> Literal {
        self.lit_stack.pop().unwrap()
    }


    fn move_scope(&mut self, name: String) {
        self.current_scope = name;
    }

    fn scope_name(&self) -> String {
        self.current_scope.clone()
    }

    fn drop_scope(&mut self, name: String) -> Arc<Mutex<ContainingScope>> {
        self.scopes.remove(&name).unwrap()
    }

    fn get_scope(&self, name: String) -> &Arc<Mutex<ContainingScope>> {
        self.scopes.get(&name).unwrap()
    }

}

impl Visitor for Vm {
    fn visit<V>(&mut self, visitable: &mut V)
    where
        V: Visitable,
    {
        visitable
            .visit(self)
            .expect("Found errors while visiting token!")
    }


    fn process(&mut self) {
        while let Some(tk) = &mut self.tks.pop_back() {
            self.visit(tk)
        }
    }

    fn process_until(&mut self, until: usize) {
        let mut amount = 0;
        let actual_amount = if until <= 0 { until } else { until - 1 };
        while let Some(tk) = &mut self.tks.pop_front() {
            if amount > actual_amount {
                self.tks.push_front(tk.to_owned());
                return;
            }
            self.visit(tk);
            amount += 1;
        }
    }

    fn process_between(&mut self, from: usize, to: usize) {
        let t = self.tks.range(from..to);
        let mut another = Clone::clone(self);
        another.tks = VecDeque::from(t.map(|it| it.to_owned()).collect::<Vec<Token>>());
        another.process();
    }
}
