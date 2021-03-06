use crate::tks::{Ident, Literal, Token, TokenChain};
use crate::visit::{Scope, Visitable, Visitor};
use crate::vm::Transmute;
use anyhow::bail;
use std::io::Cursor;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Keyword {
    Export,   // export
    Import,   // import
    Let,      // let
    Const,    // const
    Function, // fn
    Return,   // return
}

impl Transmute for Keyword {
    fn size(&mut self) -> usize {
        1
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            Keyword::Export => 0x01,
            Keyword::Import => 0x02,
            Keyword::Let => 0x03,
            Keyword::Const => 0x04,
            Keyword::Function => 0x05,
            Keyword::Return => 0x06,
        }
        .write(buf)
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(match u8::read(buf)? {
            0x01 => Keyword::Export,
            0x02 => Keyword::Import,
            0x03 => Keyword::Let,
            0x04 => Keyword::Const,
            0x05 => Keyword::Function,
            0x06 => Keyword::Return,
            _ => panic!("Invalid keyword type provided!"),
        })
    }
}

impl Visitable for Keyword {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()>
    where
        V: Visitor,
    {
        match *self {
            Keyword::Export => {
                if let Literal::Ident(name) = &mut visitor.next_token()?.as_lit_no_ident(visitor, "Expected an element to export!") {
                    visitor.export(name.to_owned());
                } else {
                    bail!("Expected an ident to be exported!")
                }
            }
            Keyword::Import => {
                if let Literal::Ident(name) = &mut visitor.next_token()?.as_lit_no_ident(visitor, "Expected an element to import!") {
                    let (scope, name) = name.rsplit_once("::").unwrap();
                    visitor.import(
                        scope.to_string(),
                        name.to_string(),
                    );
                } else {
                    bail!("Expected an ident to be imported!")
                }
            }
            Keyword::Let => {
                if let Literal::Ident(name) =
                    &mut visitor.next_token()?.as_lit("Expected a variable name!")
                {
                    let value = visitor
                        .next_token()?
                        .as_lit_advanced(visitor, "Expected a variable value!");
                    visitor.add_var(name.to_owned(), value)
                } else {
                    panic!("Expected an ident name for variable!")
                }
            }
            Keyword::Const => {
                if let Literal::Ident(name) =
                    &mut visitor.next_token()?.as_lit("Expected a variable name!")
                {
                    let value = visitor
                        .next_token()?
                        .as_lit_advanced(visitor, "Expected a variable value!");
                    visitor.add_const(name.to_owned(), value);
                }
            }
            Keyword::Function => {
                if let Token::Keyword(_) = visitor.peek_token()? {
                    visitor.next_token()?;
                }
                let out_ty = if let Literal::TypeName(name) = visitor
                    .next_token()?
                    .as_lit("Expected a function output type!")
                {
                    name
                } else {
                    panic!("Expected a type name of function's output type!")
                };
                let pop = visitor.next_token()?.as_lit("Expected a function name!");
                if let Literal::Ident(name) = pop {
                    let _lparen = visitor.next_token()?;
                    let mut param_names: Vec<Ident> = vec![];
                    while visitor
                        .peek_token()
                        .expect("Unexpected end of token chain!")
                        != Token::RParen
                    {
                        let tk = visitor
                            .next_token()
                            .expect("Unexpected end of token chain!");
                        let lit = match tk {
                            Token::Literal(ref lit) => match lit {
                                Literal::Ident(name) => name,
                                _ => panic!(
                                    "Did not expect literal {:?} at function declaration!",
                                    tk
                                ),
                            },
                            _ => panic!("Did not expect token {:?} at function declaration!", tk),
                        };
                        param_names.push(lit.to_owned())
                    }
                    let _rparen = visitor.next_token()?;
                    let _lbracket = visitor.next_token()?;
                    let mut chain = TokenChain::new();
                    while visitor.peek_token().expect("Unexpected end of token chain")
                        != Token::RBracket
                    {
                        chain.push(visitor.next_token().unwrap());
                    }
                    let _rbracket = visitor.next_token()?;

                    // TODO: possible table handling?
                    visitor.add_static_fn(name, out_ty, param_names, chain);
                } else if let Literal::String(_native) = pop {
                    if let Literal::Ident(_name) = &mut visitor.pop_stack() {
                        panic!("Native functions are not yet supported!")
                    } else {
                        panic!("Expected a name for an extern function!")
                    }
                }
            }
            Keyword::Return => {
                let tk = visitor.next_token()?;
                let lit = match tk {
                    Token::Literal(lit) => lit,
                    Token::Expression(expr) => {
                        expr.clone().visit(visitor)?;
                        visitor.pop_stack()
                    }
                    _ => panic!("Expected a literal or expression!"),
                };
                visitor.push_stack(lit)
            }
        }
        Ok(())
    }
}
