use std::io::Cursor;
use crate::tks::Literal;
use crate::vm::AllocSized;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
#[repr(C)]
pub struct Constant {
    value: Literal
}

impl Constant {
    pub fn wrap(value: Literal) -> Self {
        Constant {
            value
        }
    }
}

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

    pub fn value(&mut self) -> Literal {
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