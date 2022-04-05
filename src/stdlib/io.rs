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
    let mut params = params.clone();
    let mut pattern = unwrap_args!(params => (String));
    params.pop();
    for v in params {
        pattern = pattern.replacen("{}", &v.to_string(), 1);
    };
    Literal::String(pattern)
}

#[doc(hidden)]
pub fn __io_feature<V>(visitor: &mut V) where V: Visitor {
    extern_fns!(visitor {
        scope "std::io" {
            extern fn print(value) -> void;
            extern fn println(value) -> void;
            extern fn fmt(value) -> str;
        }
    });
}