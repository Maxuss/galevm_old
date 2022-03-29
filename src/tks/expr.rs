use crate::builtin::find_builtin;
use crate::fns::Parameters;
use crate::tks::{BinaryOp, Ident, Literal, Token, TokenChain, UnaryOp};
use crate::var::ScopedValue;
use crate::visit::{Visitable, Visitor};
use crate::vm::AllocSized;
use anyhow::bail;
use std::io::Cursor;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    BinaryOp(BinaryOp, Token, Token),
    UnaryOp(UnaryOp, Token),
    StaticAccess(Vec<Ident>),
    InstanceAccess(Vec<Ident>),
    InvokeStatic(Ident, TokenChain),
    InvokeInstance(Ident, TokenChain),
    InvokeBuiltin(Ident, TokenChain),
}

impl AllocSized for Expression {
    fn size(&mut self) -> usize {
        0x01 + match self {
            Expression::BinaryOp(op, l, r) => op.size() + l.size() + r.size(),
            Expression::UnaryOp(op, l) => op.size() + l.size(),
            Expression::StaticAccess(i) => i.size(),
            Expression::InstanceAccess(i) => i.size(),
            Expression::InvokeStatic(i, p) => i.size() + p.size(),
            Expression::InvokeInstance(i, p) => i.size() + p.size(),
            Expression::InvokeBuiltin(i, p) => i.size() + p.size(),
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
            Expression::InstanceAccess(i) => {
                0x02u8.write(buf)?;
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
            Expression::InvokeBuiltin(i, p) => {
                0x05u8.write(buf)?;
                i.write(buf)?;
                p.write(buf)?;
            }
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
            0x02 => Expression::InstanceAccess(Vec::read(buf)?),
            0x03 => Expression::InvokeStatic(Ident::read(buf)?, TokenChain::read(buf)?),
            0x04 => Expression::InvokeInstance(Ident::read(buf)?, TokenChain::read(buf)?),
            0x05 => Expression::InvokeBuiltin(Ident::read(buf)?, TokenChain::read(buf)?),
            _ => bail!("Invalid expression provided!"),
        })
    }
}

//#region bits + bools
macro_rules! _sh_impl {
    ($visitor:ident $oper:tt $lh:ident $rh:ident) => {
        let lh = match $lh {
            Token::Literal(lit) => {
                lit.to_owned()
            }
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Invalid operand provided!")
        };
        let rh = match $rh {
            Token::Literal(lit) => {
                lit.to_owned()
            }
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Invalid operand provided!")
        };
        let mut lh = if let Literal::Ident(name) = lh {
            $visitor.resolve_any_var(name.as_str())
        } else {
            lh
        };
        let rh = if let Literal::Ident(name) = rh {
            $visitor.resolve_any_var(name.as_str())
        } else {
            rh
        };
        let l = match &mut lh {
            Literal::Number(lb) => {
                if let Literal::Number(rb) = rh {
                    Literal::Number(*lb $oper rb)
                } else {
                    panic!("Invalid operation provided!")
                }
            }
            _ => panic!("Invalid operand provided!")
        };
        $visitor.push_stack(l);
    }
}

macro_rules! _bit_impl {
    ($visitor:ident $oper:tt $lh:ident $rh:ident) => {
        let lh = match $lh {
            Token::Literal(lit) => {
                lit.to_owned()
            }
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Invalid operand provided!")
        };
        let rh = match $rh {
            Token::Literal(lit) => {
                lit.to_owned()
            }
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Invalid operand provided!")
        };
        let mut lh = if let Literal::Ident(name) = lh {
            $visitor.resolve_any_var(name.as_str())
        } else {
            lh
        };
        let rh = if let Literal::Ident(name) = rh {
            $visitor.resolve_any_var(name.as_str())
        } else {
            rh
        };
        let l = match &mut lh {
            Literal::Bool(lb) => {
                if let Literal::Bool(rb) = rh {
                    Literal::Bool(*lb $oper rb)
                } else {
                    panic!("Invalid operand provided!")
                }
            }
            _ => panic!("Invalid operand provided!")
        };
        $visitor.push_stack(l);
    }
}

macro_rules! _bool_impl {
    ($visitor:ident $oper:tt $lh:ident $rh:ident) => {
        let lh = match $lh {
            Token::Literal(lit) => {
                lit.to_owned()
            }
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Invalid operand provided!")
        };
        let rh = match $rh {
            Token::Literal(lit) => {
                lit.to_owned()
            }
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Invalid operand provided!")
        };
        let mut lh = if let Literal::Ident(name) = lh {
            $visitor.resolve_any_var(name.as_str())
        } else {
            lh
        };
        let rh = if let Literal::Ident(name) = rh {
            $visitor.resolve_any_var(name.as_str())
        } else {
            rh
        };
        match &mut lh {
            Literal::Bool(lb) => {
                if let Literal::Bool(rb) = rh {
                    Literal::Bool(*lb $oper rb)
                } else {
                    panic!("Invalid operand provided!")
                }
            }
            _ => panic!("Invalid operand provided!")
        }
    }
}
//#endregion bits + bools
//#region binary expr impl
macro_rules! _bin_expr_impl {
    ($($str:literal)? $visitor:ident $oper:tt $lh:ident $rh:ident) => {
        let lh = match $lh {
            Token::Literal(lit) => {
                lit.to_owned()
            }
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Invalid operand provided!")
        };
        let rh = match $rh {
            Token::Literal(lit) => {
                lit.to_owned()
            }
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Invalid operand provided!")
        };
        _visit_impl!($($str)? $visitor $oper lh, rh);
    };
}

macro_rules! _visit_impl {
    ($($str:literal)? $visitor:ident $oper:tt $lh:ident, $rh:ident) => {
        let mut lh = if let Literal::Ident(name) = $lh {
            $visitor.resolve_any_var(name.to_owned().as_str())
        } else {
            $lh
        };
        let rh = if let Literal::Ident(name) = $rh {
            $visitor.resolve_any_var(name.to_owned().as_str())
        } else {
            $rh
        };
        let d = match &mut lh {
            $(
            Literal::String(str) => {
                let _ = $str;
                match rh {
                    Literal::Number(num) => {
                        Literal::String(str.to_owned() $oper &num.to_string())
                    }
                    Literal::Float(f) => {
                        Literal::String(str.to_owned() $oper &f.to_string())
                    }
                    Literal::String(rstr) => {
                        Literal::String(str.to_owned() $oper &rstr)
                    }
                    Literal::Char(c) => {
                        Literal::String(str.to_owned() $oper &c.to_string())
                    }
                    _ => panic!("Invalid operand provided!")
                }
            }
            )?
            Literal::Number(lnum) => {
                if let Literal::Number(rnum) = rh {
                    Literal::Number(*lnum $oper rnum)
                } else {
                    panic!("Invalid operand provided!")
                }
            }
            Literal::Float(f) => {
                if let Literal::Float(rnum) = rh {
                    Literal::Float(*f $oper rnum)
                } else {
                    panic!("Invalid operand provided!")
                }
            }
            $(
            Literal::Char(c) => {
                let _ = $str;
                if let Literal::Char(ch) = rh {
                    Literal::String(c.to_string() $oper &ch.to_string())
                } else {
                    panic!("Invalid operand provided!")
                }
            }
            )?
            _ => panic!("Invalid operand provided!")
        };
        $visitor.push_stack(d);
    };
}
//#endregion binary expr impl

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
            Expression::BinaryOp(op, lh, rh) => match op {
                BinaryOp::Assign => {
                    let lh = lh.as_lit("Expected a variable name to set!");
                    if let Literal::Ident(lh) = lh {
                        let rh = match rh {
                            Token::Literal(lit) => lit.to_owned(),
                            Token::Expression(expr) => {
                                expr.visit(visitor)?;
                                visitor.pop_stack()
                            }
                            _ => panic!("Invalid operand provided!"),
                        };
                        visitor.add_var(lh, rh);
                    } else {
                        panic!("Expected a variable name to set!")
                    }
                }
                BinaryOp::Add => {
                    _bin_expr_impl!("" visitor + lh rh);
                }
                BinaryOp::Sub => {
                    _bin_expr_impl!(visitor - lh rh);
                }
                BinaryOp::Div => {
                    _bin_expr_impl!(visitor / lh rh);
                }
                BinaryOp::Mul => {
                    _bin_expr_impl!(visitor * lh rh);
                }
                BinaryOp::Mod => {
                    _bin_expr_impl!(visitor % lh rh);
                }
                BinaryOp::And => {
                    _bool_impl!(visitor && lh rh);
                }
                BinaryOp::Or => {
                    _bool_impl!(visitor || lh rh);
                }
                BinaryOp::Eq => {
                    _bool_impl!(visitor == lh rh);
                }
                BinaryOp::Neq => {
                    _bool_impl!(visitor != lh rh);
                }
                BinaryOp::BitAnd => {
                    _bit_impl!(visitor & lh rh);
                }
                BinaryOp::BitOr => {
                    _bit_impl!(visitor | lh rh);
                }
                BinaryOp::BitXor => {
                    _bit_impl!(visitor ^ lh rh);
                }
                BinaryOp::BitRsh => {
                    _sh_impl!(visitor >> lh rh);
                }
                BinaryOp::BitLsh => {
                    _sh_impl!(visitor << lh rh);
                }
            },
            Expression::UnaryOp(op, v) => match op {
                UnaryOp::Neg => {
                    let lit = _tk2lit!(v visitor);
                    let l = match lit {
                        Literal::Bool(b) => Literal::Bool(!b),
                        _ => panic!("Invalid literal provided!"),
                    };
                    visitor.push_stack(l);
                }
                UnaryOp::Rev => {
                    let lit = _tk2lit!(v visitor);
                    let l = match lit {
                        Literal::Number(num) => Literal::Number(-num),
                        Literal::Float(f) => Literal::Float(-f),
                        _ => panic!("Invalid literal provided!"),
                    };
                    visitor.push_stack(l);
                }
            },
            Expression::StaticAccess(path) => {
                if path.len() > 1 {
                    // specific scope
                    let scope = path.get(0).unwrap();
                    let element = path.get(1).unwrap();
                    let scope = visitor.get_scope(scope.to_owned());
                    let lit = match scope.lock().unwrap().get_any_value(&element).unwrap() {
                        ScopedValue::Constant(v) => v,
                        ScopedValue::Mutable(v) => v,
                        _ => Literal::Void,
                    };
                    visitor.push_stack(lit);
                } else {
                    // all scopes
                    let element = path.get(0).unwrap();
                    let lit = visitor.resolve_any_var(&element);
                    visitor.push_stack(lit);
                }
            }
            Expression::InstanceAccess(_path) => {
                // TODO
            }
            Expression::InvokeStatic(path, params) => {
                let lit = visitor.call_static_fn(path.to_owned(), params.to_vec());
                visitor.push_stack(lit);
            }
            Expression::InvokeInstance(path, params) => {
                let old_params = params.clone();
                let params = params
                    .iter_mut()
                    .map(|it| it.as_lit_advanced(visitor, "Expected a literal-like!"))
                    .collect::<Parameters>();
                let str = params.get(0).unwrap();
                let str = match str {
                    Literal::Struct(str) => str.to_owned(),
                    Literal::Number(ptr) => Box::new(
                        visitor
                            .read_dynamic(*ptr as usize)
                            .expect(&format!("Did not find structure at {}", ptr)),
                    ),
                    _ => panic!(
                        "Expected a structure or structure pointer, got {:?}!",
                        str.clone()
                    ),
                };
                let lit = visitor.call_inst_fn(path.to_owned(), str, old_params);
                visitor.push_stack(lit);
            }
            Expression::InvokeBuiltin(name, params) => {
                let lit = find_builtin(name.to_owned()).call((params
                    .iter_mut()
                    .map(|it| it.as_lit_advanced(visitor, "Expected a literal-like!"))
                    .collect::<Parameters>(),));
                visitor.push_stack(lit);
            }
        }
        Ok(())
    }
}
