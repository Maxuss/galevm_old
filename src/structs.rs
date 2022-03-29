use crate::fns::Parameters;
use crate::tks::{Literal, TokenChain};
use crate::var::ContainingScope;
use crate::visit::Visitor;
use crate::vm::AllocSized;
use std::io::Cursor;

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct Structure {
    typename: String,
    scope: ContainingScope,
}

impl AllocSized for Structure {
    fn size(&mut self) -> usize {
        self.typename.size() + self.scope.size()
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.typename.write(buf)?;
        self.scope.write(buf)?;
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Structure {
            typename: String::read(buf)?,
            scope: ContainingScope::read(buf)?,
        })
    }
}

impl Structure {
    pub fn typename(&self) -> String {
        self.typename.clone()
    }

    pub fn with_type(ty: &str) -> Self {
        Self {
            typename: ty.to_string(),
            scope: ContainingScope::new(),
        }
    }

    pub fn add_var(&mut self, name: String, var: Literal) {
        self.scope.add_var(&name, var);
    }

    pub fn add_const(&mut self, name: String, var: Literal) {
        self.scope.add_const(&name, var)
    }

    pub fn add_inst_fn(
        &mut self,
        name: String,
        output_ty: String,
        param_names: Vec<String>,
        tks: TokenChain,
    ) {
        self.scope.add_inst_fn(&name, output_ty, param_names, tks);
    }

    pub fn add_static_fn(
        &mut self,
        name: String,
        output_ty: String,
        param_names: Vec<String>,
        tks: TokenChain,
    ) {
        self.scope.add_static_fn(&name, output_ty, param_names, tks);
    }

    pub fn call_inst_fn<V>(
        &mut self,
        name: String,
        this: Box<Structure>,
        params: Parameters,
        visitor: &mut V,
    ) -> Literal
    where
        V: Visitor,
    {
        let fnc = self.scope.get_inst_fn(&name).expect(&format!(
            "Could not find instance function {} in current scope!",
            name
        ));
        fnc.call(this, params, visitor)
    }

    pub fn call_static_fn<V>(
        &mut self,
        name: String,
        params: Parameters,
        visitor: &mut V,
    ) -> Literal
    where
        V: Visitor,
    {
        let fnc = self.scope.get_static_fn(&name).expect(&format!(
            "Could not find function {} in current scope!",
            name
        ));
        fnc.call(params, visitor)
    }
}
