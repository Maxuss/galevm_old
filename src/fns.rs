use crate::structs::Structure;
use crate::tks::{Literal, TokenChain};
use crate::var::ContainingScope;
use crate::visit::{Scope, Visitor};
use crate::vm::Transmute;
use rand::RngCore;
use std::fmt::Debug;
use std::io::Cursor;

pub type Parameters = Vec<Literal>;

#[derive(Debug, Clone, PartialEq)]
pub struct InstFn {
    out_ty: String,
    param_names: Vec<String>,
    chain: TokenChain,
}

impl Transmute for InstFn {
    fn size(&mut self) -> usize {
        self.out_ty.size() + self.param_names.len() + 4 + self.chain.len()
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.out_ty.write(buf)?;
        self.param_names.write(buf)?;
        self.chain.write(buf)?;
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(InstFn::new(
            String::read(buf)?,
            Vec::read(buf)?,
            TokenChain::read(buf)?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticFn {
    out_ty: String,
    param_names: Vec<String>,
    chain: TokenChain,
}

impl Transmute for StaticFn {
    fn size(&mut self) -> usize {
        self.out_ty.size() + self.param_names.len() + 4 + self.chain.len()
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.out_ty.write(buf)?;
        self.param_names.write(buf)?;
        self.chain.write(buf)?;
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(StaticFn::new(
            String::read(buf)?,
            Vec::read(buf)?,
            TokenChain::read(buf)?,
        ))
    }
}

impl InstFn {
    pub fn new(out_ty: String, param_names: Vec<String>, chain: TokenChain) -> Self {
        Self {
            out_ty,
            param_names,
            chain,
        }
    }

    pub fn call<V>(&self, this: Box<Structure>, params: Parameters, visitor: &mut V) -> Literal
    where
        V: Visitor,
    {
        if params.len() != self.param_names.len() {
            panic!(
                "Invalid amount of arguments supplied! Expected {} args!",
                self.param_names.len()
            );
        };

        // preparing scope and injecting arguments
        let mut scope = ContainingScope::new();
        for pid in 0..self.param_names.len() {
            scope.add_const(self.param_names[pid].as_str(), params[pid].to_owned());
        }
        scope.add_const("this", Literal::Struct(this));

        // creating scope
        let cached = visitor.scope_name();
        let name = format!("inst_fn_0x{:2x}", rand::thread_rng().next_u64());
        visitor.push_scope_level(Scope::InstanceFunction);
        visitor.push_scope(name.clone(), scope);

        visitor.move_scope(name.clone());

        // processing tokens
        visitor.load_chain(&mut self.chain.clone());
        let output = visitor.pop_stack();
        if !output.type_str(&self.out_ty) {
            panic!(
                "Invalid output provided! Expected output of type {:?}",
                self.out_ty
            )
        };

        // changing scopes back
        visitor.move_scope(cached);
        visitor.drop_scope(name);
        visitor.pop_scope_level();

        output
    }
}

impl StaticFn {
    pub fn new(out_ty: String, param_names: Vec<String>, chain: TokenChain) -> Self {
        Self {
            out_ty,
            param_names,
            chain,
        }
    }

    pub fn call<V>(&self, params: Parameters, visitor: &mut V) -> Literal
    where
        V: Visitor,
    {
        if params.len() != self.param_names.len() {
            panic!(
                "Invalid amount of arguments supplied! Expected {} arg(s)!",
                self.param_names.len()
            );
        };

        // preparing scope and injecting arguments
        let mut scope = ContainingScope::new();
        for pid in 0..self.param_names.len() {
            scope.add_const(self.param_names[pid].as_str(), params[pid].to_owned());
        }

        // creating scope
        let cached = visitor.scope_name();
        let name = format!("static_fn_0x{:2x}", rand::thread_rng().next_u64());
        visitor.push_scope_level(Scope::InstanceFunction);
        visitor.push_scope(name.clone(), scope);

        visitor.move_scope(name.clone());

        // processing tokens
        visitor.load_chain(&mut self.chain.clone());
        visitor.process();
        let output = visitor.pop_stack();
        if !output.type_str(&self.out_ty) {
            panic!(
                "Invalid output provided! Expected output of type {:?}",
                self.out_ty
            )
        };

        // changing scopes back
        visitor.move_scope(cached);
        visitor.drop_scope(name);
        visitor.pop_scope_level();

        output
    }
}