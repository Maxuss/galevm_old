use std::collections::HashMap;
use crate::fns::{import_globals, InstFn, Parameters};
use crate::tks::{Literal, TokenChain};
use crate::var::{ContainingScope, merge_scopes};
use crate::visit::{Scope, Visitor};
use crate::vm::Transmute;
use std::io::Cursor;
use rand::RngCore;

/// Structure template, to actually access inner data requires the [StructureInstance]
#[derive(Debug, Clone, PartialEq)]
pub struct StructureTemplate {
    typename: String,
    inst_vars: HashMap<String, String>,
    static_vars: HashMap<String, Literal>,
    inst_fns: HashMap<String, Box<InstFn>>,
    scope: ContainingScope
}

/// Structure instance, actually holding its data
#[derive(Debug, Clone, PartialEq)]
pub struct StructureInstance {
    typename: String,
    template_ptr: usize,
    inst_vars: HashMap<String, Literal>
}

impl Transmute for StructureInstance {
    fn size(&mut self) -> usize {
        self.typename.size() + (self.template_ptr as u64).size() + self.inst_vars.size()
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.typename.write(buf)?;
        (self.template_ptr as u64).write(buf)?;
        self.inst_vars.write(buf)?;
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self> where Self: Sized {
        Ok(Self {
            typename: String::read(buf)?,
            template_ptr: u64::read(buf)? as usize,
            inst_vars: HashMap::read(buf)?
        })
    }
}

impl Transmute for StructureTemplate {
    fn size(&mut self) -> usize {
        self.typename.size() + self.inst_vars.size() + self.static_vars.size() + self.inst_fns.size() + self.scope.size()
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.typename.write(buf)?;
        self.inst_vars.write(buf)?;
        self.static_vars.write(buf)?;
        self.inst_fns.write(buf)?;
        self.scope.write(buf)?;
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(StructureTemplate {
            typename: String::read(buf)?,
            inst_vars: HashMap::read(buf)?,
            static_vars: HashMap::read(buf)?,
            inst_fns: HashMap::read(buf)?,
            scope: ContainingScope::read(buf)?
        })
    }
}

impl StructureTemplate {
    pub fn typename(&self) -> String {
        self.typename.clone()
    }

    pub fn with_type(ty: &str) -> Self {
        Self {
            typename: ty.to_string(),
            inst_vars: Default::default(),
            static_vars: Default::default(),
            inst_fns: Default::default(),
            scope: ContainingScope::new()
        }
    }

    pub fn from_chain<V>(name: String, chain: TokenChain, visitor: &mut V) -> Self where V: Visitor {
        let mut scope = ContainingScope::new();
        import_globals(&mut scope, visitor);
        scope.add_const("$name", Literal::String(name.clone()));

        let mut this = Self {
            typename: name.clone(),
            inst_vars: Default::default(),
            static_vars: Default::default(),
            inst_fns: Default::default(),
            scope: scope.clone()
        };
        visitor.register_type(&this);

        let cached = visitor.scope_name();
        let sc_name = format!("struct_0x{:2x}", rand::thread_rng().next_u64());
        visitor.push_scope_level(Scope::Struct);
        visitor.push_scope(sc_name.clone(), scope);
        visitor.move_scope(sc_name.clone());

        // processing tokens
        let mut new_chain = chain.clone();
        visitor.load_chain(&mut new_chain);
        visitor.process_between(0, new_chain.len());
        visitor.move_scope(cached);
        let scope = visitor.drop_scope(sc_name);
        let mut scope = scope.lock().unwrap();
        visitor.pop_scope_level();

        merge_scopes(&mut this.scope, &mut scope);
        this
    }

    pub fn get_static_var(&self, name: String) -> Option<Literal> {
        self.scope.get_var(&name)
    }

    pub fn get_const(&self, name: String) -> Literal {
        self.scope.get_const(&name).expect(&format!("Could not find static constant {} in struct {}", name, self.typename))
    }

    pub fn get_inst_var(&mut self, this: &StructureInstance, name: String) -> Literal {
        this.inst_vars.get(&name). expect(&format!("Could not find variable {} in struct {}", name, self.typename)).to_owned()
    }

    pub fn reassign_var(&mut self, this: &mut StructureInstance, name: String, new: Literal) {
        if self.inst_vars.contains_key(&name) && new.type_str(self.inst_vars.get(&name).unwrap()) {
            this.inst_vars.insert(name, new);
        }
    }

    pub fn add_inst_var(&mut self, this: &mut StructureInstance, name: String, value: Literal) {
        this.inst_vars.insert(name, value);
    }

    pub fn add_static_var(&mut self, name: String, var: Literal) {
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
        self.inst_fns.insert(name, Box::new(InstFn::new(output_ty, param_names, tks)));
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
        this: StructureInstance,
        name: String,
        params: Parameters,
        visitor: &mut V,
    ) -> Literal
    where
        V: Visitor,
    {
        let fnc = self.inst_fns.get(&name).expect(&format!(
            "Could not find instance function {} in struct {}!",
            name,
            self.typename
        ));
        fnc.call(Box::new(this), params, visitor)
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
        fnc.call(params, Some(visitor))
    }
}

impl StructureInstance {
    pub fn from_template<V>(template: &StructureTemplate, visitor: &mut V) -> Self where V: Visitor {
        Self {
            typename: template.typename.clone(),
            template_ptr: visitor.get_type_ptr(template.typename.clone()).unwrap(),
            inst_vars: Default::default()
        }
    }

    pub fn from_ptr<V>(ptr: usize, visitor: &mut V) -> Self where V: Visitor {
        let str = visitor.resolve_type_raw(ptr);

        Self {
            typename: str.typename,
            template_ptr: ptr,
            inst_vars: Default::default()
        }
    }

    pub fn typename(&self) -> String {
        self.typename.clone()
    }
}