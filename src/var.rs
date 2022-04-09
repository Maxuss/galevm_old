use crate::fns::{ExternFn, InstFn, StaticFn};
use crate::structs::Structure;
use crate::tks::{Literal, TokenChain};
use crate::vm::Transmute;
use std::collections::HashMap;
use std::io::Cursor;
use std::mem;

fn _string_size(str: &String) -> usize {
    str.len() + 2
}

fn _write_str(str: &String, buf: &mut Vec<u8>) -> anyhow::Result<()> {
    let bytes = str.as_bytes();
    let vec: [u8; 2] = unsafe { mem::transmute(str.len() as u16) };
    buf.extend_from_slice(&vec);
    buf.extend_from_slice(&bytes);
    Ok(())
}

impl<V> Transmute for HashMap<String, V>
where
    V: Transmute,
{
    fn size(&mut self) -> usize {
        let mut s = 0;
        let size: Vec<u64> = self
            .iter_mut()
            .map(|(k, v)| ((_string_size(k) + v.size()) as u64))
            .collect();
        for i in size {
            s += i;
        }
        (s + 4) as usize
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        (self.len() as u32).write(buf)?;

        for (k, v) in self {
            _write_str(k, buf)?;
            v.write(buf)?;
        }
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let len = u32::read(buf)? as usize;
        let mut map = HashMap::new();
        for _ in 0..len {
            let key = String::read(buf)?;
            let value = V::read(buf)?;
            map.insert(key, value).unwrap();
        }
        Ok(map)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct ContainingScope {
    mutables: HashMap<String, Literal>,
    consts: HashMap<String, Literal>,
    static_fns: HashMap<String, Box<StaticFn>>,
    inst_fns: HashMap<String, Box<InstFn>>,
    exports: Vec<String>,
    extern_fns: HashMap<String, Box<ExternFn>>,
    imports: HashMap<String, Vec<String>>,
    structs: HashMap<String, Box<Structure>>,
}

impl Transmute for ContainingScope {
    fn size(&mut self) -> usize {
        self.mutables.size()
            + self.consts.size()
            + self.static_fns.size()
            + self.inst_fns.size()
            + self.exports.size()
            + self.imports.size()
            + self.structs.size()
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        self.mutables.write(buf)?;
        self.consts.write(buf)?;
        self.consts.write(buf)?;
        self.static_fns.write(buf)?;
        self.inst_fns.write(buf)?;
        self.exports.write(buf)?;
        self.imports.write(buf)?;
        self.structs.write(buf)?;
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(ContainingScope {
            mutables: HashMap::read(buf)?,
            consts: HashMap::read(buf)?,
            static_fns: HashMap::read(buf)?,
            inst_fns: HashMap::read(buf)?,
            exports: Vec::read(buf)?,
            extern_fns: HashMap::read(buf)?,
            imports: HashMap::read(buf)?,
            structs: HashMap::read(buf)?,
        })
    }
}

impl ContainingScope {
    pub fn new() -> Self {
        Self {
            mutables: Default::default(),
            consts: Default::default(),
            static_fns: Default::default(),
            inst_fns: Default::default(),
            exports: vec![],
            extern_fns: Default::default(),
            imports: Default::default(),
            structs: Default::default(),
        }
    }

    pub fn add_var(&mut self, name: &str, var: Literal) {
        if self.mutables.contains_key(name) {
            self.mutables.remove(name);
        }
        self.mutables.insert(name.to_string(), var);
    }

    pub fn add_const(&mut self, name: &str, var: Literal) {
        if self.consts.contains_key(name) {
            panic!("Can not reassign constant {}!", name)
        }
        self.consts.insert(name.to_string(), var);
    }

    pub fn mutate(&mut self, name: &str, var: Literal) {
        if self.mutables.get(name).unwrap().type_matches(&var) {
            self.mutables.remove(name);
            self.mutables.insert(name.to_string(), var);
        } else {
            panic!("Tried to mutate variable of different type!")
        }
    }

    pub fn get_var(&self, name: &str) -> Option<Literal> {
        self.mutables.get(name).map(|l| l.to_owned())
    }

    pub fn get_const(&self, name: &str) -> Option<Literal> {
        self.consts.get(name).map(|l| l.to_owned())
    }

    pub fn export(&mut self, export: &str) {
        self.exports.push(export.to_string())
    }

    pub fn import(&mut self, from: &str, import: &str) {
        if self.imports.contains_key(from) {
            let mut imports = self.imports.remove(from).unwrap().clone();
            imports.push(import.to_string());
            self.imports.insert(from.to_owned(), imports);
        } else {
            self.imports
                .insert(from.to_string(), vec![import.to_string()]);
        };
    }

    pub fn add_static_fn(
        &mut self,
        name: &str,
        output_ty: String,
        param_names: Vec<String>,
        tks: TokenChain,
    ) {
        self.static_fns.insert(
            name.to_string(),
            Box::new(StaticFn::new(output_ty, param_names, tks)),
        );
    }

    pub fn add_inst_fn(
        &mut self,
        name: &str,
        output_ty: String,
        param_names: Vec<String>,
        tks: TokenChain,
    ) {
        self.inst_fns.insert(
            name.to_string(),
            Box::new(InstFn::new(output_ty, param_names, tks)),
        );
    }

    pub fn add_extern_fn(
        &mut self,
        name: &str,
        output_ty: String,
        param_names: Vec<String>,
        handler_ptr: usize
    ) {
        self.extern_fns.insert(name.to_string(),
         Box::new(ExternFn::new(output_ty, param_names, handler_ptr)));
    }

    pub fn add_prebuilt_static_fn(&mut self, name: &str, sf: StaticFn) {
        self.static_fns.insert(name.to_string(), Box::new(sf));
    }

    pub fn add_prebuilt_inst_fn(&mut self, name: &str, inf: InstFn) {
        self.inst_fns.insert(name.to_string(), Box::new(inf));
    }

    pub fn add_prebuilt_extern_fn(&mut self, name: &str, ef: ExternFn) {
        self.extern_fns.insert(name.to_string(), Box::new(ef));
    }

    pub fn add_struct(&mut self, name: &str, typ: Structure) {
        self.structs.insert(name.to_string(), Box::new(typ));
    }

    pub fn get_struct(&mut self, typename: &str) -> Option<Structure> {
        self.structs.get(typename).map(|s| (**s).clone())
    }

    pub fn get_static_fn(&mut self, name: &str) -> Option<StaticFn> {
        self.static_fns.get(name).map(|f| *f.clone())
    }

    pub fn get_inst_fn(&mut self, name: &str) -> Option<InstFn> {
        self.inst_fns.get(name).map(|f| *f.clone())
    }

    pub fn get_extern_fn(&mut self, name: &str) -> Option<ExternFn> {
        self.extern_fns.get(name).map(|f| *f.clone())
    }

    pub fn get_any_value(&mut self, name: &str) -> Option<ScopedValue> {
        let c = self.get_const(name);
        if c.is_some() {
            return Some(ScopedValue::Constant(c?));
        }
        let m = self.get_var(name);
        if m.is_some() {
            return Some(ScopedValue::Mutable(c?));
        }
        let s = self.get_struct(name);
        if s.is_some() {
            return Some(ScopedValue::Type(s?));
        }
        let sf = self.get_static_fn(name);
        if sf.is_some() {
            return Some(ScopedValue::StaticFn(sf?));
        }
        let inf = self.get_inst_fn(name);
        if inf.is_some() {
            return Some(ScopedValue::InstFn(inf?));
        }
        let ef = self.get_extern_fn(name);
        if ef.is_some() {
            return Some(ScopedValue::ExternFn(ef?))
        }
        return None;
    }

    pub fn imports(&mut self) -> HashMap<String, Vec<String>> {
        self.imports.to_owned()
    }
}

#[derive(Debug, Clone)]
pub enum ScopedValue {
    Constant(Literal),
    Mutable(Literal),
    Type(Structure),
    StaticFn(StaticFn),
    InstFn(InstFn),
    ExternFn(ExternFn)
}
