use std::io::Cursor;
use crate::tks::{Literal, Token};
use crate::var::{Mutable, VariableScope};
use crate::vm::{AllocSized, ConstSized, Memory};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Scope {
    Struct,
    StaticFunction,
    InstanceFunction
}

pub trait TokenProvider {
    fn next_token(&mut self) -> anyhow::Result<Token>;
    fn peek_token(&mut self) -> anyhow::Result<Token>;
}

pub trait Visitor: Memory + TokenProvider {
    fn push_stack(&mut self, value: Literal);
    fn pop_stack(&mut self) -> Literal;
    fn resolve_var(&self, name: &str) -> anyhow::Result<&Mutable>;
    fn resolve_const(&self, name: &str) -> anyhow::Result<&Literal>;
    fn push_scope(&mut self, scope: Scope);
    fn pop_scope(&mut self) -> Scope;
    fn visit<V>(&mut self, visitable: &mut V) where V: Visitable;

    fn add_var(&mut self, name: String, var: Mutable);
    fn add_const(&mut self, name: String, var: Literal);

    fn resolve_any_var(&self, name: &str) -> Literal {
        let var = self.resolve_var(name);
        if var.is_ok() {
            var.unwrap().value()
        } else {
            self.resolve_const(name).unwrap().to_owned()
        }
    }

    fn process_chain<V>(&mut self, chain: &mut Vec<V>) where V: Visitable {
        for ele in chain {
            self.visit(ele);
        };
    }
}

pub trait Visitable {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()> where V: Visitor;
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Vm {
    mem: Vec<u8>,
    free: usize,
    pos: usize,
    tks: Vec<Token>,
    lit_stack: Vec<Literal>,
    scope: VariableScope,
    scope_types: Vec<Scope>
}

impl Vm {
    pub fn new() -> Self {
        Self {
            mem: vec![],
            free: 0,
            pos: 0,
            tks: vec![],
            lit_stack: vec![],
            scope: VariableScope::new(),
            scope_types: vec![]
        }
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

    fn write<A>(&mut self, ptr: usize, value: &mut A) -> anyhow::Result<()> where A: AllocSized {
        self.jump(ptr)?;
        let size = value.size();
        let mut slice = self.mem[ptr..ptr+size].to_vec();
        value.write(&mut slice)?;
        self.mem.splice(ptr..ptr+size, slice);
        self.free -= size;
        Ok(())
    }

    fn alloc_write<A>(&mut self, value: &mut A) -> anyhow::Result<usize> where A: AllocSized {
        self.alloc(value.size())?;
        self.write(self.pos, value)?;
        Ok(self.pos)
    }

    fn read_dynamic<A>(&mut self, ptr: usize) -> anyhow::Result<A> where A: AllocSized {
        let mut slice = Cursor::new(self.mem[ptr..].to_vec());
        self.jump(ptr)?;
        let value = A::read(&mut slice)?;
        drop(slice);
        Ok(value)
    }

    fn read_const<A>(&mut self, ptr: usize) -> anyhow::Result<A> where A: ConstSized + AllocSized {
        let mut slice = Cursor::new(self.mem[ptr..(ptr + A::const_size())].to_vec());
        self.jump(ptr)?;
        let value = A::read(&mut slice)?;
        drop(slice);
        Ok(value)
    }

    fn free(&mut self, ptr: usize, amount: usize) -> anyhow::Result<()> {
        self.mem.drain(ptr..ptr+amount);
        Ok(())
    }
}

impl TokenProvider for Vm {
    fn next_token(&mut self) -> anyhow::Result<Token> {
        Ok(self.tks.pop().unwrap())
    }

    fn peek_token(&mut self) -> anyhow::Result<Token> {
        Ok(self.tks.iter().peekable().peek().unwrap().to_owned().to_owned())
    }
}

impl Visitor for Vm {
    fn push_stack(&mut self, value: Literal) {
        self.lit_stack.push(value);
    }

    fn pop_stack(&mut self) -> Literal {
        self.lit_stack.pop().unwrap()
    }

    fn resolve_var(&self, name: &str) -> anyhow::Result<&Mutable> {
        self.scope.get_var(name)
    }

    fn resolve_const(&self, name: &str) -> anyhow::Result<&Literal> {
        self.scope.get_const(name)
    }

    fn push_scope(&mut self, scope: Scope) {
        self.scope_types.push(scope);
    }

    fn pop_scope(&mut self) -> Scope {
        self.scope_types.pop().unwrap()
    }

    fn visit<V>(&mut self, visitable: &mut V) where V: Visitable {
        visitable.visit(self).expect("Found errors while visiting token!")
    }

    fn add_var(&mut self, name: String, var: Mutable) {
        self.scope.add_var(&name, var.value())
    }

    fn add_const(&mut self, name: String, var: Literal) {
        self.scope.add_const(&name, var)
    }
}