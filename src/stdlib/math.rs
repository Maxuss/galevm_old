use crate::{extern_fns, Parameters, unwrap_args};
use crate::tks::Literal;
use crate::visit::Visitor;

fn min(params: Parameters) -> Literal {
    let (min, value) = unwrap_args!(params => (Number, Number));
    Literal::Number(if min < value { value } else { min })
}

fn max(params: Parameters) -> Literal {
    let (max, value) = unwrap_args!(params => (Number, Number));
    Literal::Number(if max < value { max } else { value })
}

fn minf(params: Parameters) -> Literal {
    let (min, value) = unwrap_args!(params => (Float, Float));
    Literal::Float(if min < value { value } else { min })
}

fn maxf(params: Parameters) -> Literal {
    let (max, value) = unwrap_args!(params => (Float, Float));
    Literal::Float(if max < value { max } else { value })
}

fn pow(params: Parameters) -> Literal {
    let (value, pow) = unwrap_args!(params => (Number, Number));
    Literal::Number(value.pow(pow as u32))
}

fn powf(params: Parameters) -> Literal {
    let (value, pow) = unwrap_args!(params => (Float, Number));
    Literal::Float(value.powi(pow as i32))
}

fn cmp(params: Parameters) -> Literal {
    let (lh, rh) = unwrap_args!(params => (Number, Number));
    Literal::Number(if lh == rh { 0 } else if lh < rh { -1 } else { 1 })
}

fn cmpf(params: Parameters) -> Literal {
    let (lh, rh) = unwrap_args!(params => (Float, Float));
    Literal::Number(if lh == rh { 0 } else if lh < rh { -1 } else { 1 })
}

fn sin(params: Parameters) -> Literal {
    let val = unwrap_args!(params => (Float));
    Literal::Float(val.sin())
}

fn cos(params: Parameters) -> Literal {
    let val = unwrap_args!(params => (Float));
    Literal::Float(val.cos())
}

fn tan(params: Parameters) -> Literal {
    let val = unwrap_args!(params => (Float));
    Literal::Float(val.tan())
}

#[doc(hidden)]
pub fn __math_feature<V>(visitor: &mut V) where V: Visitor {
    extern_fns!(visitor {
        scope "std::math" {
            extern fn min(min, val) -> num;
            extern fn max(max, val) -> num;
            extern fn pow(value, pow) -> num;
            extern fn cmp(first, second) -> num;

            extern fn minf(min, val) -> float;
            extern fn maxf(max, val) -> float;
            extern fn powf(value, pow) -> float;
            extern fn cmpf(first, second) -> num;

            extern fn sin(value) -> float;
            extern fn cos(value) -> float;
            extern fn tan(value) -> float;
        }
    })
}