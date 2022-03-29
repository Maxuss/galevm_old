use crate::structs::Structure;
use crate::tks::{Literal, Token, TokenChain};
use crate::var::{ContainingScope, ScopedValue};
use crate::vm::{AllocSized, ConstSized, Memory};
use crate::ToResult;
use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

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
}

pub trait Visitor: Memory + TokenProvider + Clone {
    fn push_stack(&mut self, value: Literal);
    fn pop_stack(&mut self) -> Literal;
    fn resolve_var(&self, name: &str) -> anyhow::Result<Literal>;
    fn resolve_const(&self, name: &str) -> anyhow::Result<Literal>;
    fn push_scope_level(&mut self, scope: Scope);
    fn pop_scope_level(&mut self) -> Scope;
    fn scope_level(&mut self) -> Scope;
    fn push_scope(&mut self, name: String, scope: ContainingScope);
    fn visit<V>(&mut self, visitable: &mut V)
    where
        V: Visitable;

    fn import(&mut self, from: String, name: String);
    fn export(&mut self, name: String);

    fn add_var(&mut self, name: String, var: Literal);
    fn add_const(&mut self, name: String, var: Literal);

    fn move_scope(&mut self, name: String);
    fn scope_name(&self) -> String;
    fn drop_scope(&mut self, name: String);

    fn get_scope(&self, name: String) -> &Arc<Mutex<ContainingScope>>;

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

    fn register_type(&mut self, name: String, structure: Structure);
    fn resolve_type(&mut self, name: String) -> Structure;

    fn call_inst_fn(&mut self, name: String, this: Box<Structure>, params: TokenChain) -> Literal;
    fn call_static_fn(&mut self, name: String, params: TokenChain) -> Literal;

    fn process(&mut self);

    fn resolve_any_var(&self, name: &str) -> Literal {
        let var = self.resolve_var(name);
        if var.is_ok() {
            var.unwrap().to_owned()
        } else {
            self.resolve_const(name).unwrap().to_owned()
        }
    }

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
    mem: Vec<u8>,
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
            mem: vec![],
            free: 0,
            pos: 0,
            tks: VecDeque::new(),
            lit_stack: vec![],
            current_scope: "__GLOBAL".to_string(),
            scopes: HashMap::from([(
                "__GLOBAL".to_string(),
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

impl Memory for Vm {
    fn jump(&mut self, pos: usize) -> anyhow::Result<()> {
        self.pos = pos;
        Ok(())
    }

    fn alloc(&mut self, amount: usize) -> anyhow::Result<usize> {
        self.mem.extend(vec![0u8; amount]);
        self.free += amount;
        Ok(self.pos)
    }

    fn write<A>(&mut self, ptr: usize, value: &mut A) -> anyhow::Result<()>
    where
        A: AllocSized,
    {
        self.jump(ptr)?;
        let size = value.size();
        let mut slice = self.mem[ptr..ptr + size].to_vec();
        value.write(&mut slice)?;
        self.mem.splice(ptr..ptr + size, slice);
        self.free -= size;
        Ok(())
    }

    fn alloc_write<A>(&mut self, value: &mut A) -> anyhow::Result<usize>
    where
        A: AllocSized,
    {
        self.alloc(value.size())?;
        self.write(self.pos, value)?;
        Ok(self.pos)
    }

    fn read_dynamic<A>(&mut self, ptr: usize) -> anyhow::Result<A>
    where
        A: AllocSized,
    {
        let mut slice = Cursor::new(self.mem[ptr..].to_vec());
        self.jump(ptr)?;
        let value = A::read(&mut slice)?;
        drop(slice);
        Ok(value)
    }

    fn read_const<A>(&mut self, ptr: usize) -> anyhow::Result<A>
    where
        A: ConstSized + AllocSized,
    {
        let mut slice = Cursor::new(self.mem[ptr..(ptr + A::const_size())].to_vec());
        self.jump(ptr)?;
        let value = A::read(&mut slice)?;
        drop(slice);
        Ok(value)
    }

    fn free(&mut self, ptr: usize, amount: usize) -> anyhow::Result<()> {
        self.mem.drain(ptr..ptr + amount);
        Ok(())
    }
}

impl TokenProvider for Vm {
    fn next_token(&mut self) -> anyhow::Result<Token> {
        Ok(self.tks.pop_back().unwrap())
    }

    fn peek_token(&mut self) -> anyhow::Result<Token> {
        Ok(self
            .tks
            .iter()
            .rev()
            .peekable()
            .peek()
            .unwrap()
            .to_owned()
            .to_owned())
    }

    fn add_token(&mut self, tk: Token) {
        self.tks.push_front(tk)
    }
}

impl Visitor for Vm {
    fn push_stack(&mut self, value: Literal) {
        self.lit_stack.push(value);
    }

    fn pop_stack(&mut self) -> Literal {
        self.lit_stack.pop().unwrap()
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

    fn visit<V>(&mut self, visitable: &mut V)
    where
        V: Visitable,
    {
        visitable
            .visit(self)
            .expect("Found errors while visiting token!")
    }

    fn import(&mut self, from: String, name: String) {
        self.scopes.get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap().import(&from, &name);
    }

    fn export(&mut self, name: String) {
        self.scopes.get(&self.current_scope)
            .unwrap()
            .lock()
            .unwrap().export(&name);
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
            let split = name.split("::").collect::<Vec<&str>>();
            let scope = split.get(0).unwrap().to_string();
            let fnc = split.get(1).unwrap().to_string();
            let fnc = self.get_scope(scope.to_owned()).lock().unwrap().get_static_fn(&fnc).unwrap();
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

    fn process(&mut self) {
        while let Some(tk) = &mut self.tks.pop_back() {
            self.visit(tk)
        }
    }
}
