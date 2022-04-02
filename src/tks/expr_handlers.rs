use anyhow::bail;
use crate::tks::{BinaryOp, Literal, Token};
use crate::visit::{Visitable, Visitor};

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

macro_rules! _lt_gt_impl {
    ($visitor:ident $oper:tt $lh:ident $rh:ident) => {
        let lh = $lh.as_lit_advanced($visitor, "Expected a literal-like!");
        let rh = $rh.as_lit_advanced($visitor, "Expected a literal-like!");
        let mut lh = if let Literal::Ident(name) = lh {
            $visitor.resolve_any_var(name.to_owned().as_str())
        } else {
            lh
        };
        let rh = if let Literal::Ident(name) = rh {
            $visitor.resolve_any_var(name.to_owned().as_str())
        } else {
            rh
        };
        let d = match &mut lh {
            Literal::String(str) => {
                match rh {
                    Literal::Number(num) => {
                        Literal::Bool(str.to_owned() $oper num.to_string())
                    }
                    Literal::Float(f) => {
                        Literal::Bool(str.to_owned() $oper f.to_string())
                    }
                    Literal::String(rstr) => {
                        Literal::Bool(str.to_owned() $oper rstr)
                    }
                    Literal::Char(c) => {
                        Literal::Bool(str.to_owned() $oper c.to_string())
                    }
                    _ => panic!("Invalid operand provided!")
                }
            }
            Literal::Number(lnum) => {
                if let Literal::Number(rnum) = rh {
                    Literal::Bool(*lnum $oper rnum)
                } else {
                    panic!("Invalid operand provided!")
                }
            }
            Literal::Float(f) => {
                if let Literal::Float(rnum) = rh {
                    Literal::Bool(*f $oper rnum)
                } else {
                    panic!("Invalid operand provided!")
                }
            }
            _ => panic!("Invalid operand provided!")
        };
        $visitor.push_stack(d);
    }
}
//#endregion binary expr impl

pub(crate) fn _binary_op_handler<V>(visitor: &mut V, op: &mut BinaryOp, lh: &mut Token, rh: &mut Token) -> anyhow::Result<()> where V: Visitor {
    match op {
        BinaryOp::Assign => {
            let lh = lh.as_lit("Expected a variable name to set!");
            if let Literal::Ident(lh) = lh {
                let rh = match rh {
                    Token::Literal(lit) => lit.to_owned(),
                    Token::Expression(expr) => {
                        expr.visit(visitor)?;
                        visitor.pop_stack()
                    }
                    _ => bail!("Invalid operand provided!"),
                };
                visitor.add_var(lh, rh);
            } else {
                bail!("Expected a variable name to set!")
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
        BinaryOp::Lt => {
            _lt_gt_impl!(visitor < lh rh);
        }
        BinaryOp::Gt => {
            _lt_gt_impl!(visitor > lh rh);
        }
    };
    Ok(())
}