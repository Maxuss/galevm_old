use std::collections::VecDeque;
use crate::{extern_fns, Parameters, unwrap_args};
use crate::tks::Literal;
use crate::visit::Visitor;

fn print(params: Parameters) -> Literal {
    let val = unwrap_args!(params => (String));
    print!("{}", val);
    Literal::Void
}

fn println(params: Parameters) -> Literal {
    let val = unwrap_args!(params => (String));
    println!("{}", val);
    Literal::Void
}

fn fmt(params: Parameters) -> Literal {
    let mut params = VecDeque::from(params);
    let mut pattern = match params.pop_front().unwrap() {
        Literal::String(str) => str,
        _ => panic!("Expected string literal!")
    };
    for v in params {
        pattern = pattern.replacen("{}", &v.to_string(), 1);
    };
    Literal::String(pattern.to_string())
}

fn debug(params: Parameters) -> Literal {
    let value = params[0].to_owned();
    match value {
        Literal::Number(v) => println!("{}", v),
        Literal::Float(v) => println!("{}", v),
        Literal::String(v) => println!("{}", v),
        Literal::Char(v) => println!("{}", v),
        Literal::Ident(v) => println!("${}", v),
        Literal::Bool(v) => println!("{}", v),
        Literal::TypeName(v) => println!("type {}", v),
        Literal::Struct(str) => println!("{:?}", str),
        Literal::Void => println!("void")
    };
    Literal::Void
}

fn debugp(params: Parameters) -> Literal {
    let value = params[0].to_owned();
    match value {
        Literal::Number(v) => println!("{}", v),
        Literal::Float(v) => println!("{}", v),
        Literal::String(v) => println!("{}", v),
        Literal::Char(v) => println!("{}", v),
        Literal::Ident(v) => println!("${}", v),
        Literal::Bool(v) => println!("{}", v),
        Literal::TypeName(v) => println!("type {}", v),
        Literal::Struct(str) => println!("{:#?}", str),
        Literal::Void => println!("void")
    };
    Literal::Void
}

#[doc(hidden)]
pub fn __io_feature<V>(visitor: &mut V) where V: Visitor {
    extern_fns!(visitor {
        scope "std::io" {
            extern fn print(value) -> void;
            extern fn println(value) -> void;
            extern fn fmt(pattern, varargs) -> str;
            extern fn debug(value) -> void;
            extern fn debugp(value) -> void;
        }
    });
}