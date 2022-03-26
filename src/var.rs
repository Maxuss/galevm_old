use std::collections::HashMap;
use std::io::Cursor;
use crate::tks::Literal;
use crate::vm::AllocSized;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
#[repr(C)]
pub struct Mutable {
    value: Literal
}

impl Mutable {
    pub fn wrap(value: Literal) -> Self {
        Mutable {
            value
        }
    }

    pub fn mutate(&mut self, value: Literal) -> Self {
        if self.value.type_matches(&value) {
            self.value = value;
        };
        self.clone()
    }

    pub fn value(&self) -> Literal {
        self.value.clone()
    }
}

impl AllocSized for Mutable {
    fn size(&mut self) -> usize {
        match &self.value {
            Literal::Number(_) => {
                8
            }
            Literal::Float(_) => {
                8
            }
            Literal::String(s) => {
                s.len() + 2
            }
            Literal::Char(_) => {
                4
            }
            Literal::Ident(i) => {
                i.len() + 2
            }
            Literal::TypeName(tt) => {
                tt.len() + 2
            }
            Literal::Bool(_) => {
                1
            }
        }
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        match &mut self.value {
            Literal::Number(n) => {
                n.write(buf)
            }
            Literal::Float(f) => {
                f.write(buf)
            }
            Literal::String(s) => {
                s.write(buf)
            }
            Literal::Char(c) => {
                c.write(buf)
            }
            Literal::Ident(i) => {
                i.write(buf)
            }
            Literal::TypeName(tt) => {
                tt.write(buf)
            }
            Literal::Bool(bool) => {
                bool.write(buf)
            }
        }

    }

    fn read(_: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self> where Self: Sized {
        panic!("Mutables can not be read raw!")
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct VariableScope {
    mutables: HashMap<String, Mutable>,
    consts: HashMap<String, Literal>
}

impl VariableScope {
    pub fn new() -> Self {
        Self {
            mutables: Default::default(),
            consts: Default::default()
        }
    }

    pub fn add_var(&mut self, name: &str, var: Literal) {
        self.mutables.insert(name.to_string(), Mutable::wrap(var));
    }

    pub fn add_const(&mut self, name: &str, var: Literal) {
        self.consts.insert(name.to_string(), var);
    }

    pub fn get_var(&self, name: &str) -> anyhow::Result<&Mutable> {
        Ok(self.mutables.get(name).unwrap())
    }

    pub fn get_const(&self, name: &str) -> anyhow::Result<&Literal> {
        Ok(self.consts.get(name).unwrap())
    }
}