use crate::var::Mutable;
use crate::visit::{Visitable, Visitor};

pub type Ident = String;
pub type TokenChain = Vec<Token>;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
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
    End
}

impl Visitable for Token {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()> where V: Visitor {
        match self {
            Token::Literal(literal) => {
                literal.visit(visitor)
            }
            Token::Keyword(kw) => {
                kw.visit(visitor)
            }
            Token::Expression(expr) => {
                expr.visit(visitor)
            }
            Token::End => {
                // We should not reach this!
                panic!("Tried to move to END scope!")
            }
            _ => {
                Ok(())
            } // ignoring because it is either scopes or whitespaces
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Literal {
    Number(i64),
    Float(f64),
    String(String),
    Char(char),
    Ident(Ident),
    Bool(bool),
    TypeName(String)
}

impl Literal {
    pub fn type_matches(&self, other: &Literal) -> bool {
        match *self {
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
        }
    }
}

impl Visitable for Literal {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()> where V: Visitor {
        visitor.push_stack(self.to_owned());
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Keyword {
    Struct,   // struct
    Export,   // export
    Import,   // import
    Let,      // let
    Const,    // const
    Function, // fn
}

impl Visitable for Keyword {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()> where V: Visitor {
        match *self {
            Keyword::Struct => {}
            Keyword::Export => {}
            Keyword::Import => {}
            Keyword::Let => {
                if let Literal::Ident(name) = &mut visitor.pop_stack() {
                    visitor.pop_stack().visit(visitor)?;
                    let mut value = visitor.pop_stack();
                    match &mut value {
                        Literal::Number(num) => {
                            visitor.alloc_write(num)?
                        }
                        Literal::Float(float) => {
                            visitor.alloc_write(float)?
                        }
                        Literal::String(str) => {
                            visitor.alloc_write(str)?
                        }
                        Literal::Char(char) => {
                            visitor.alloc_write(char)?
                        }
                        Literal::Ident(ident) => {
                            visitor.alloc_write(ident)?
                        }
                        Literal::TypeName(tt) => {
                            visitor.alloc_write(tt)?
                        }
                        Literal::Bool(bool) => {
                            visitor.alloc_write(bool)?
                        }
                    };
                    visitor.add_var(name.to_owned(), Mutable::wrap(value))
                } else {
                    panic!("Expected an ident name for variable!")
                }
            }
            Keyword::Const => {
                if let Literal::Ident(name) = &mut visitor.pop_stack() {
                    visitor.pop_stack().visit(visitor)?;
                    let mut value = visitor.pop_stack();
                    match &mut value {
                        Literal::Number(num) => {
                            visitor.alloc_write(num)?
                        }
                        Literal::Float(float) => {
                            visitor.alloc_write(float)?
                        }
                        Literal::String(str) => {
                            visitor.alloc_write(str)?
                        }
                        Literal::Char(char) => {
                            visitor.alloc_write(char)?
                        }
                        Literal::Ident(ident) => {
                            visitor.alloc_write(ident)?
                        }
                        Literal::TypeName(tt) => {
                            visitor.alloc_write(tt)?
                        }
                        Literal::Bool(bool) => {
                            visitor.alloc_write(bool)?
                        }
                    };
                    visitor.add_const(name.to_owned(), value);
                }
            }
            Keyword::Function => {

            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Expression {
    BinaryOp(BinaryOp, Token, Token),
    UnaryOp(UnaryOp, Token),
    StaticAccess(Vec<Ident>),
    InstanceAccess(Vec<Ident>),
    InvokeFunction(Vec<Ident>, TokenChain),
    InvokeBuiltin(Ident, TokenChain)
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
            Token::Literal(lit) => {
                lit.to_owned()
            }
            Token::Expression(expr) => {
                expr.visit($visitor)?;
                $visitor.pop_stack()
            }
            _ => panic!("Expected literal, got {:?}", $v)
        }
    };
}

impl Visitable for Expression {
    fn visit<V>(&mut self, visitor: &mut V) -> anyhow::Result<()> where V: Visitor {
        match self {
            Expression::BinaryOp(op, lh, rh) => {
                match op {
                    BinaryOp::Assign => {} // ignoring, should not occur normally
                    BinaryOp::Add => { _bin_expr_impl!("" visitor + lh rh); }
                    BinaryOp::Sub => { _bin_expr_impl!(visitor - lh rh); }
                    BinaryOp::Div => { _bin_expr_impl!(visitor / lh rh); }
                    BinaryOp::Mul => { _bin_expr_impl!(visitor * lh rh); }
                    BinaryOp::Mod => { _bin_expr_impl!(visitor % lh rh); }
                    BinaryOp::And => { _bool_impl!(visitor && lh rh); }
                    BinaryOp::Or => { _bool_impl!(visitor || lh rh); }
                    BinaryOp::BitAnd => { _bit_impl!(visitor & lh rh); }
                    BinaryOp::BitOr => { _bit_impl!(visitor | lh rh); }
                    BinaryOp::BitXor => { _bit_impl!(visitor ^ lh rh); }
                    BinaryOp::BitRsh => { _sh_impl!(visitor >> lh rh); }
                    BinaryOp::BitLsh => { _sh_impl!(visitor << lh rh); }
                }
            }
            Expression::UnaryOp(op, v) => {
                match op {
                    UnaryOp::Neg => {
                        let lit = _tk2lit!(v visitor);
                        let l = match lit {

                            Literal::Bool(b) => {
                                Literal::Bool(!b)
                            }
                            _ => panic!("Invalid literal provided!")
                        };
                        visitor.push_stack(l);
                    }
                    UnaryOp::Rev => {
                        let lit = _tk2lit!(v visitor);
                        let l = match lit {
                            Literal::Number(num) => {
                                Literal::Number(-num)
                            }
                            Literal::Float(f) => {
                                Literal::Float(-f)
                            }
                            _ => panic!("Invalid literal provided!")
                        };
                        visitor.push_stack(l);
                    }
                }
            }
            Expression::StaticAccess(_path) => {
                // TODO
            }
            Expression::InstanceAccess(_path) => {
                // TODO
            }
            Expression::InvokeFunction(_path, _params) => {
                // TODO
            }
            Expression::InvokeBuiltin(_name, _params) => {
                // TODO
            }
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum BinaryOp {
    Assign, // =, unused by default
    Add,    // +
    Sub,    // -
    Div,    // /
    Mul,    // *
    Mod,    //  %
    And,    // &&
    Or,     // ||
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    BitRsh, // >>
    BitLsh  // <<
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum UnaryOp {
    Neg, // !
    Rev, // ~
}