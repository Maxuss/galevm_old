use crate::tks::{Literal, Token};
use crate::var::Mutable;
use crate::vm::Memory;

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
    fn resolve_var(&mut self, name: &str) -> anyhow::Result<Mutable>;
    fn resolve_const(&mut self, name: &str) -> anyhow::Result<Literal>;
    fn push_scope(&mut self, scope: Scope);
    fn pop_scope(&mut self) -> Scope;
    fn visit<V>(&mut self, visitable: V) where V: Visitable;

    fn resolve_any_var(&mut self, name: &str) -> Literal {
        let var = self.resolve_var(name);
        if var.is_ok() {
            var.unwrap().value()
        } else {
            self.resolve_const(name).unwrap()
        }
    }
}

pub trait Visitable {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()> where V: Visitor;
}