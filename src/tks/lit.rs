use crate::structs::Structure;
use crate::tks::Ident;
use crate::visit::{Visitable, Visitor};
use crate::vm::Transmute;
use std::fmt::{Display, Formatter};
use std::io::Cursor;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(i64),
    Float(f64),
    String(String),
    Char(char),
    Ident(Ident),
    Bool(bool),
    TypeName(String),
    Struct(Box<Structure>),
    Void,
}

impl Transmute for Literal {
    fn size(&mut self) -> usize {
        1 + match self {
            Literal::Number(v) => v.size(),
            Literal::Float(v) => v.size(),
            Literal::String(v) => v.size(),
            Literal::Char(v) => v.size(),
            Literal::Ident(v) => v.size(),
            Literal::Bool(v) => v.size(),
            Literal::TypeName(v) => v.size(),
            Literal::Struct(v) => v.size(),
            Literal::Void => 0,
        }
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            Literal::Number(v) => {
                0x01u8.write(buf)?;
                v.write(buf)?
            }
            Literal::Float(v) => {
                0x02u8.write(buf)?;
                v.write(buf)?
            }
            Literal::String(v) => {
                0x03u8.write(buf)?;
                v.write(buf)?
            }
            Literal::Char(v) => {
                0x04u8.write(buf)?;
                v.write(buf)?
            }
            Literal::Ident(v) => {
                0x05u8.write(buf)?;
                v.write(buf)?
            }
            Literal::Bool(v) => {
                0x06u8.write(buf)?;
                v.write(buf)?
            }
            Literal::TypeName(v) => {
                0x07u8.write(buf)?;
                v.write(buf)?
            }
            Literal::Struct(v) => {
                0x08u8.write(buf)?;
                v.write(buf)?
            }
            Literal::Void => 0x00u8.write(buf)?,
        };
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(match u8::read(buf)? {
            0x00 => Literal::Void,
            0x01 => Literal::Number(i64::read(buf)?),
            0x02 => Literal::Float(f64::read(buf)?),
            0x03 => Literal::String(String::read(buf)?),
            0x04 => Literal::Char(char::read(buf)?),
            0x05 => Literal::Ident(Ident::read(buf)?),
            0x06 => Literal::Bool(bool::read(buf)?),
            0x07 => Literal::TypeName(String::read(buf)?),
            0x08 => Literal::Struct(Box::new(Structure::read(buf)?)),
            _ => panic!("Invalid LitID provided!"),
        })
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Number(v) => f.write_str(&v.to_string()),
            Literal::Float(v) => f.write_str(&v.to_string()),
            Literal::String(v) => f.write_str(&v),
            Literal::Char(v) => f.write_str(&v.to_string()),
            Literal::Ident(v) => f.write_str(&v),
            Literal::Bool(v) => f.write_str(&v.to_string()),
            Literal::TypeName(v) => f.write_str(&v),
            Literal::Struct(v) => f.write_str(&format!("{:#?}", v)),
            Literal::Void => f.write_str("*"),
        }
    }
}

impl Literal {
    pub fn this_type(&self) -> String {
        match self {
            Literal::Number(_) => "num",
            Literal::Float(_) => "float",
            Literal::String(_) => "str",
            Literal::Char(_) => "char",
            Literal::Ident(_) => "void",
            Literal::Bool(_) => "bool",
            Literal::TypeName(_) => "typename",
            Literal::Struct(str) => str.typename(),
            Literal::Void => "void"
        }.to_string()
    }

    pub fn type_str(&self, tn: &str) -> bool {
        match self {
            Literal::Number(_) => tn == "num",
            Literal::Float(_) => tn == "float",
            Literal::String(_) => tn == "str",
            Literal::Char(_) => tn == "char",
            Literal::Ident(_) => true,
            Literal::Bool(_) => tn == "bool",
            Literal::TypeName(_) => tn == "typename",
            Literal::Void => tn == "void",
            Literal::Struct(str) => tn == &str.typename(),
        }
    }

    pub fn type_matches(&self, other: &Literal) -> bool {
        match self {
            Literal::Number(_) => {
                if let Literal::Number(_) = other {
                    true
                } else {
                    false
                }
            }
            Literal::Float(_) => {
                if let Literal::Float(_) = other {
                    true
                } else {
                    false
                }
            }
            Literal::String(_) => {
                if let Literal::String(_) = other {
                    true
                } else {
                    false
                }
            }
            Literal::Char(_) => {
                if let Literal::Char(_) = other {
                    true
                } else {
                    false
                }
            }
            Literal::Ident(_) => {
                if let Literal::Ident(_) = other {
                    true
                } else {
                    false
                }
            }
            Literal::TypeName(_) => {
                if let Literal::TypeName(_) = other {
                    true
                } else {
                    false
                }
            }
            Literal::Bool(_) => {
                if let Literal::Bool(_) = other {
                    true
                } else {
                    false
                }
            }
            Literal::Struct(str) => {
                if let Literal::Struct(another) = other {
                    another.typename() == str.typename()
                } else {
                    false
                }
            }
            _ => true,
        }
    }
}

impl Visitable for Literal {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()>
    where
        V: Visitor,
    {
        visitor.push_stack(self.to_owned());
        Ok(())
    }
}
