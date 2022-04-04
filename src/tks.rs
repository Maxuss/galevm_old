mod expr;
pub(crate) mod expr_handlers;
mod kw;
mod lit;
mod ops;

pub use expr::*;
pub use kw::*;
pub use lit::*;
pub use ops::*;

use crate::visit::{Visitable, Visitor};
use crate::vm::Transmute;
use anyhow::bail;
use std::io::Cursor;

pub type Ident = String;
pub type TokenChain = Vec<Token>;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Whitespace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    LSquare,
    RSquare,
    Literal(Literal),
    Keyword(Keyword),
    Expression(Box<Expression>),
    End,
}

impl Transmute for Token {
    fn size(&mut self) -> usize {
        1 + match self {
            Token::Whitespace => 0,
            Token::LBracket => 0,
            Token::RBracket => 0,
            Token::LParen => 0,
            Token::RParen => 0,
            Token::LSquare => 0,
            Token::RSquare => 0,
            Token::Literal(lit) => lit.size(),
            Token::Keyword(kw) => kw.size(),
            Token::Expression(expr) => expr.size(),
            Token::End => 0,
        }
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            Token::Whitespace => 0u8.write(buf)?,
            Token::LBracket => 0x01u8.write(buf)?,
            Token::RBracket => 0x02u8.write(buf)?,
            Token::LParen => 0x03u8.write(buf)?,
            Token::RParen => 0x04u8.write(buf)?,
            Token::LSquare => 0x05u8.write(buf)?,
            Token::RSquare => 0x06u8.write(buf)?,
            Token::Literal(l) => {
                0x07u8.write(buf)?;
                l.write(buf)?;
            }
            Token::Keyword(kw) => {
                0x08u8.write(buf)?;
                kw.write(buf)?;
            }
            Token::Expression(expr) => {
                0x09u8.write(buf)?;
                expr.write(buf)?;
            }
            Token::End => {
                0x0Au8.write(buf)?;
            }
        };
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(match u8::read(buf)? {
            0x00 => Token::Whitespace,
            0x01 => Token::LBracket,
            0x02 => Token::RBracket,
            0x03 => Token::LParen,
            0x04 => Token::RParen,
            0x05 => Token::LSquare,
            0x06 => Token::RSquare,
            0x07 => Token::Literal(Literal::read(buf)?),
            0x08 => Token::Keyword(Keyword::read(buf)?),
            0x09 => Token::Expression(Box::new(Expression::read(buf)?)),
            0x0A => Token::End,
            _ => bail!("Invalid token provided!"),
        })
    }
}

impl Visitable for Token {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()>
    where
        V: Visitor,
    {
        match self {
            Token::Literal(literal) => literal.visit(visitor),
            Token::Keyword(kw) => kw.visit(visitor),
            Token::Expression(expr) => expr.visit(visitor),
            Token::End => {
                // We should not reach this!
                panic!("Tried to move to END scope!")
            }
            _ => Ok(()), // ignoring because it is either scopes or whitespaces
        }
    }
}

impl Token {
    pub fn as_lit_advanced<V>(&mut self, visitor: &mut V, panic_msg: &str) -> Literal
    where
        V: Visitor,
    {
        match self {
            Token::Literal(lit) => match lit {
                Literal::Ident(id) => visitor.resolve_any_var(id.as_str()),
                _ => lit.to_owned(),
            },
            Token::Expression(expr) => {
                expr.visit(visitor).unwrap();
                visitor.pop_stack()
            }
            _ => panic!("{}", panic_msg),
        }
    }

    pub fn as_lit_no_ident<V>(&mut self, visitor: &mut V, panic_msg: &str) -> Literal
    where
        V: Visitor,
    {
        match self {
            Token::Literal(lit) => lit.to_owned(),
            Token::Expression(expr) => {
                expr.visit(visitor).unwrap();
                visitor.pop_stack()
            }
            _ => panic!("{}", panic_msg),
        }
    }

    pub fn as_lit(&self, panic_msg: &str) -> Literal {
        match self {
            Token::Literal(l) => l.to_owned(),
            _ => panic!("{}", panic_msg),
        }
    }
}
