use std::collections::VecDeque;
use crate::tks::expr_handlers::_binary_op_handler;
use crate::tks::{BinaryOp, Ident, Literal, Token, TokenChain, UnaryOp};
use crate::visit::{Visitable, Visitor};
use crate::vm::Transmute;
use anyhow::bail;
use std::io::Cursor;
use crate::structs::StructureInstance;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    BinaryOp(BinaryOp, Token, Token),
    UnaryOp(UnaryOp, Token),
    StaticAccess(Vec<Ident>),
    InstanceAccess(Box<StructureInstance>, Vec<Ident>),
    InvokeStatic(Ident, TokenChain),
    InvokeInstance(Ident, TokenChain),
    IfStmt,
    ElseStmt,
    ElifStmt,
    WhileStmt,
}

impl Transmute for Expression {
    fn size(&mut self) -> usize {
        0x01 + match self {
            Expression::BinaryOp(op, l, r) => op.size() + l.size() + r.size(),
            Expression::UnaryOp(op, l) => op.size() + l.size(),
            Expression::StaticAccess(i) => i.size(),
            Expression::InstanceAccess(this, i) => this.size() + i.size(),
            Expression::InvokeStatic(i, p) => i.size() + p.size(),
            Expression::InvokeInstance(i, p) => i.size() + p.size(),
            _ => 0,
        }
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            Expression::BinaryOp(op, l, r) => {
                0x00u8.write(buf)?;
                op.write(buf)?;
                l.write(buf)?;
                r.write(buf)?;
            }
            Expression::UnaryOp(op, l) => {
                0x01u8.write(buf)?;
                op.write(buf)?;
                l.write(buf)?;
            }
            Expression::StaticAccess(i) => {
                0x02u8.write(buf)?;
                i.write(buf)?;
            }
            Expression::InstanceAccess(this, i) => {
                0x02u8.write(buf)?;
                this.write(buf)?;
                i.write(buf)?;
            }
            Expression::InvokeStatic(i, p) => {
                0x03u8.write(buf)?;
                i.write(buf)?;
                p.write(buf)?;
            }
            Expression::InvokeInstance(i, p) => {
                0x04u8.write(buf)?;
                i.write(buf)?;
                p.write(buf)?;
            }
            Expression::IfStmt => 0x05u8.write(buf)?,
            Expression::ElseStmt => 0x06u8.write(buf)?,
            Expression::WhileStmt => 0x07u8.write(buf)?,
            Expression::ElifStmt => 0x08u8.write(buf)?,
        };
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(match u8::read(buf)? {
            0x00 => {
                Expression::BinaryOp(BinaryOp::read(buf)?, Token::read(buf)?, Token::read(buf)?)
            }
            0x01 => Expression::UnaryOp(UnaryOp::read(buf)?, Token::read(buf)?),
            0x02 => Expression::InstanceAccess(Box::new(StructureInstance::read(buf)?), Vec::read(buf)?),
            0x03 => Expression::InvokeStatic(Ident::read(buf)?, TokenChain::read(buf)?),
            0x04 => Expression::InvokeInstance(Ident::read(buf)?, TokenChain::read(buf)?),
            0x05 => Expression::IfStmt,
            0x06 => Expression::ElseStmt,
            0x07 => Expression::WhileStmt,
            0x08 => Expression::ElifStmt,
            _ => bail!("Invalid expression provided!"),
        })
    }
}

macro_rules! _tkbool {
    ($tk:ident) => {
        match $tk {
            Literal::Number(n) => n != 0,
            Literal::Bool(b) => b,
            Literal::Void => false,
            _ => true,
        }
    };
    ($tk:expr) => {
        match $tk {
            Literal::Number(n) => n != 0,
            Literal::Bool(b) => b,
            Literal::Void => false,
            _ => true,
        }
    };
}

macro_rules! _tk2lit {
    ($v:ident $visitor:ident) => {
        match $v {
            Token::Literal(lit) => lit.to_owned(),
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Expected literal, got {:?}", $v),
        }
    };
}

impl Visitable for Expression {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()>
    where
        V: Visitor,
    {
        match self {
            Expression::BinaryOp(op, lh, rh) => _binary_op_handler(visitor, op, lh, rh),
            Expression::UnaryOp(op, v) => {
                return match op {
                    UnaryOp::Neg => {
                        let lit = _tk2lit!(v visitor);
                        let l = match lit {
                            Literal::Bool(b) => Literal::Bool(!b),
                            _ => panic!("Invalid literal provided!"),
                        };
                        visitor.push_stack(l);
                        Ok(())
                    }
                    UnaryOp::Rev => {
                        let lit = _tk2lit!(v visitor);
                        let l = match lit {
                            Literal::Number(num) => Literal::Number(-num),
                            Literal::Float(f) => Literal::Float(-f),
                            _ => panic!("Invalid literal provided!"),
                        };
                        visitor.push_stack(l);
                        Ok(())
                    }
                }
            }
            Expression::StaticAccess(path) => {
                if path.len() > 2 {
                    // specific scope
                    let scope = path.get(0).unwrap();
                    let str = path.get(1).unwrap();
                    let element = path.get(2).unwrap();
                    let vis = visitor.clone();
                    let scope = vis.get_scope(scope.to_owned());
                    let mut scope = scope.lock().unwrap();
                    let str = scope.get_struct(str).unwrap();
                    let str = str.lock().unwrap();

                    let lit = str.get_static_var(element.to_string()).unwrap_or_else(|| str.get_const(element.to_string()));
                    visitor.push_stack(lit);
                } else {
                    // all scopes
                    let str = path.get(0).unwrap();
                    let element = path.get(1).unwrap();

                    let str = visitor.resolve_type(str.to_string());
                    let str = str.lock().unwrap();
                    let lit = str.get_static_var(element.to_string()).unwrap_or_else(|| str.get_const(element.to_string()));
                    visitor.push_stack(lit);
                }
                return Ok(());
            }
            Expression::InstanceAccess(this, path) => {
                let mut path = VecDeque::from(path.to_owned());
                let name: Ident = path.pop_back().unwrap();
                let str: Ident = path.pop_back().unwrap();
                let scope = path.pop_back();
                let str = if let Some(scope) = scope {
                    visitor.get_scope(scope).lock().unwrap().get_struct(&str).unwrap()
                } else {
                    visitor.resolve_type(str)
                };
                let var = str.lock().unwrap().get_inst_var(this, name);
                visitor.push_stack(var);
                return Ok(());
            }
            Expression::InvokeStatic(path, params) => {
                let lit = visitor.call_static_fn(path.to_owned(), params.to_vec());
                visitor.push_stack(lit);
                return Ok(());
            }
            Expression::InvokeInstance(path, params) => {
                let (ident, name) = path.rsplit_once(".").unwrap();
                let this = if let Literal::Struct(this) = visitor.resolve_var(ident).unwrap() {
                    this
                } else {
                    panic!("Expected structure ident!")
                };

                let old_params = params.clone();

                let lit = visitor.call_inst_fn(name.to_string(), this, old_params);
                visitor.push_stack(lit);
                return Ok(());
            }
            Expression::IfStmt => _visit_if(visitor),
            Expression::WhileStmt => {
                let mut condition = visitor.next_token()?;

                if _tkbool!(condition.as_lit_advanced(visitor, "Could not process while condition!"))
                {
                    let _lbracket = visitor.next_token()?;
                    let mut chain = TokenChain::new();
                    while visitor.peek_token()? != Token::RBracket {
                        chain.push(visitor.next_token()?);
                    }

                    chain.reverse();
                    let _rbracket = visitor.next_token()?;

                    while _tkbool!(
                        condition.as_lit_advanced(visitor, "Could not process while condition!")
                    ) {
                        for ele in &chain {
                            visitor.insert_token(ele.to_owned(), 0);
                        }
                        visitor.process_until(chain.len());
                    }
                } else {
                    visitor.push_stack(Literal::Void);
                }
                return Ok(());
            }
            _ => bail!("Unexpected unbounded {:?} token!", self),
        }
    }
}

fn _visit_if<V>(visitor: &mut V) -> anyhow::Result<()>
where
    V: Visitor,
{
    let mut next = visitor.next_token()?;
    let next = next.as_lit_advanced(visitor, "Expected a literal-like statement!");
    let boolean = match next {
        Literal::Number(n) => n != 0,
        Literal::Bool(b) => b,
        Literal::Void => false,
        _ => true,
    };
    if boolean {
        let _lbracket = visitor.next_token()?;
        let mut chain = TokenChain::new();
        while visitor.peek_token()? != Token::RBracket {
            chain.push(visitor.next_token()?);
        }

        chain.reverse();
        let len = chain.len();
        for ele in chain {
            visitor.insert_token(ele, 0);
        }
        let _rbracket = visitor.next_token()?;

        visitor.process_until(len);

        while let Ok(_) = &mut visitor.peek_token() {
            let mut expr = visitor.peek_token()?;
            if let Token::Expression(box expr) = &mut expr {
                match expr {
                    Expression::ElifStmt => {
                        let _ = _visit_elif(visitor, true);
                    }
                    Expression::ElseStmt => {
                        _visit_else(visitor, true)?;
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
    } else {
        // consuming all the left over tokens from the `if` branch
        let _lbracket = visitor.next_token()?;
        while visitor.peek_token()? != Token::RBracket {
            let _ = visitor.next_token()?;
        }
        let _rbracket = visitor.next_token()?;
        // trying to find elif's and else's
        let mut matched = false;
        while let Ok(Token::Expression(expr)) = &mut visitor.peek_token() {
            match expr {
                box Expression::ElseStmt => {
                    _visit_else(visitor, matched)?;
                    return Ok(());
                }
                box Expression::ElifStmt => {
                    let success = _visit_elif(visitor, matched);
                    matched = success.is_ok();
                }
                _ => {
                    visitor.push_stack(Literal::Void);
                    return Ok(());
                }
            }
        }
    }
    return Ok(());
}

fn _visit_else<V>(visitor: &mut V, matched: bool) -> anyhow::Result<()>
where
    V: Visitor,
{
    // consuming current token
    let _ = visitor.next_token()?;
    let _lbracket = visitor.next_token()?;
    let mut chain = TokenChain::new();
    while visitor.peek_token()? != Token::RBracket {
        chain.push(visitor.next_token()?);
    }
    if matched {
        // `if` branch already matched before, so just dropping all of our stuff
        drop(chain);
        let _rbracket = visitor.next_token()?;
        return Ok(());
    }

    chain.reverse();
    let len = chain.len();
    for ele in chain {
        visitor.insert_token(ele, 0);
    }
    visitor.process_until(len);
    Ok(())
}

fn _visit_elif<V>(visitor: &mut V, matched: bool) -> anyhow::Result<()>
where
    V: Visitor,
{
    // consuming current token
    let _ = visitor.next_token()?;
    let mut next = visitor.next_token()?;
    let next = next.as_lit_advanced(visitor, "Expected a boolean!");
    let boolean = _tkbool!(next);
    // consuming tokens, dropping them anyways if not needed
    let _lbracket = visitor.next_token()?;
    return if matched {
        while visitor.peek_token()? != Token::RBracket {
            let _ = visitor.next_token()?;
        }
        let _rbracket = visitor.next_token()?;
        Ok(())
    } else {
        let mut chain = TokenChain::new();
        while visitor.peek_token()? != Token::RBracket {
            chain.push(visitor.next_token()?);
        }
        if boolean {
            chain.reverse();
            let len = chain.len();
            for ele in chain {
                visitor.insert_token(ele, 0);
            }

            visitor.process_until(len);

            let _rbracket = visitor.next_token()?;
            Ok(())
        } else {
            let _rbracket = visitor.next_token()?;
            bail!("exit");
        }
    };
}
