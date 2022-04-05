use crate::structs::Structure;
use crate::tks::{Literal, Token, TokenChain};
use crate::var::{ContainingScope, ScopedValue};
use crate::ToResult;
use anyhow::bail;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use crate::features::StdFeature;
use crate::fns::EXTERN_FNS;

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

    fn resolve_var(&self, name: &str) -> anyhow::Result<Literal>;
    fn resolve_const(&self, name: &str) -> anyhow::Result<Literal>;

    fn import(&mut self, from: String, name: String);
    fn export(&mut self, name: String);

    fn add_var(&mut self, name: String, var: Literal);
    fn add_const(&mut self, name: String, var: Literal);

    fn add_inst_fn(
        &mut self,
        name: String,
        output_ty: String,
        param_names: Vec<String>,
        tks: TokenChain,
    );
    fn add_static_fn(
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

    fn register_type(&mut self, name: String, structure: Structure);
    fn resolve_type(&mut self, name: String) -> Structure;

    fn call_inst_fn(&mut self, name: String, this: Box<Structure>, params: TokenChain) -> Literal;
    fn call_static_fn(&mut self, name: String, params: TokenChain) -> Literal;
    fn call_extern_fn(&mut self, name: String, params: TokenChain) -> Literal;
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
    fn drop_scope(&mut self, name: String);

    fn get_scope(&self, name: String) -> &Arc<Mutex<ContainingScope>>;
}

pub trait Visitor: TokenProvider + Clone + ScopeProvider + GlobalScope + LiteralStack {
    fn visit<V>(&mut self, visitable: &mut V)
    where
        V: Visitable;

    fn process(&mut self);

    fn process_until(&mut self, until: usize);

    fn load_chain(&mut self, chain: &mut TokenChain) {
        let iter = chain.iter();
        for ele in iter {
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
    scope_types: Vec<Scope>,
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
            scope_types: vec![Scope::Global],
        }
    }

    pub fn merged_scope(&self) -> Arc<Mutex<ContainingScope>> {
        let current = self.scopes.get(&self.current_scope).unwrap().clone();
        for (scope, values) in current.lock().unwrap().imports() {
            let scope = self.scopes.get(scope).unwrap().clone();
            for name in values {
                let value = scope.lock().unwrap().get_any_value(&name.clone());
                match value {
                    None => {
                        panic!("Tried to import non-existent value {:?}!", name)
                    }
                    Some(scoped) => match scoped {
                        ScopedValue::Constant(v) => current.lock().unwrap().add_const(name, v),
                        ScopedValue::Mutable(v) => current.lock().unwrap().add_var(name, v),
                        ScopedValue::Type(v) => current.lock().unwrap().add_struct(name, v),
                        ScopedValue::StaticFn(v) => {
                            current.lock().unwrap().add_prebuilt_static_fn(name, v)
                        }
                        ScopedValue::InstFn(v) => {
                            current.lock().unwrap().add_prebuilt_inst_fn(name, v)
                        }
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

    fn add_inst_fn(
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
            .add_inst_fn(&name, output_ty, param_names, tks);
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

    fn add_extern_fn(&mut self, name: String, output_ty: String, param_names: Vec<String>, ptr: usize) {
        self.scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .add_extern_fn(&name, output_ty, param_names, ptr);
    }

    fn register_type(&mut self, name: String, structure: Structure) {
        self.scopes
            .get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap()
            .add_struct(&name, structure);
    }

    fn resolve_type(&mut self, name: String) -> Structure {
        self.merged_scope()
            .lock()
            .unwrap()
            .get_struct(&name)
            .expect(format!("Could not find type {:?}!", name).as_str())
    }

    fn call_inst_fn(&mut self, name: String, this: Box<Structure>, params: TokenChain) -> Literal {
        let mut params = params.clone();
        let params = params
            .iter_mut()
            .map(|it| it.as_lit_advanced(self, "Expected a literal-like!"))
            .collect();
        let fnc = self
            .merged_scope()
            .lock()
            .unwrap()
            .get_inst_fn(&name)
            .expect(&format!(
                "Could not find instance function {} in current scope!",
                name
            ));
        fnc.call(this, params, self)
    }

    fn call_static_fn(&mut self, name: String, params: TokenChain) -> Literal {
        let mut params = params.clone();
        let params = params
            .iter_mut()
            .map(|it| it.as_lit_advanced(self, "Expected a literal-like!"))
            .collect();
        if name.contains("::") {
            let (scope, fnc_name) = name.rsplit_once("::").unwrap();
            let fnc = self
                .get_scope(scope.to_owned())
                .lock()
                .unwrap()
                .get_static_fn(&fnc_name)
                .unwrap();
            fnc.call(params, self)
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
            fnc.call(params, self)
        }
    }

    fn call_extern_fn(&mut self, name: String, params: TokenChain) -> Literal {
        let mut params = params.clone();
        let params = params
            .iter_mut()
            .map(|it| it.as_lit_advanced(self, "Expected a literal-like!"))
            .collect();
        if name.contains("::") {
            let (scope, fnc_name) = name.rsplit_once("::").unwrap();
            let fnc = self
                .get_scope(scope.to_owned())
                .lock()
                .unwrap()
                .get_extern_fn(&fnc_name)
                .unwrap();
            fnc.call(params)
        } else {
            let fnc = self
                .merged_scope()
                .lock()
                .unwrap()
                .get_extern_fn(&name)
                .expect(&format!(
                    "Could not find function {} in current scope!",
                    name
                ));
            fnc.call(params)
        }
    }

    fn call_ptr_fn(&mut self, ptr: usize, params: TokenChain) -> Literal {
        let fns = EXTERN_FNS.lock().unwrap();
        if fns.len() < ptr {
            panic!("Tried to call an unexistent ptr-bound external function: 0x{:2x}", ptr)
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
        self.scope_types.push(scope);
    }

    fn pop_scope_level(&mut self) -> Scope {
        self.scope_types.pop().unwrap()
    }

    fn scope_level(&mut self) -> Scope {
        *self.scope_types.last().unwrap()
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

    fn drop_scope(&mut self, name: String) {
        drop(self.scopes.remove(&name));
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
}
