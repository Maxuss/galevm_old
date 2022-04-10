use std::collections::VecDeque;
use std::io::Cursor;
use crate::{extern_fns, Parameters, unwrap_args};
use crate::structs::StructureInstance;
use crate::tks::Literal;
use crate::visit::Visitor;
use crate::vm::Transmute;

fn transmute(params: Parameters) -> Literal {
    let mut params = VecDeque::from(params);
    let mut value = params.pop_front().unwrap();
    let ty = unwrap_args!(params => (TypeName));
    let mut buf = vec![];
    value.write(&mut buf).unwrap();
    let mut cur = Cursor::new(buf);
    match ty.as_str() {
        "num" => Literal::Number(i64::read(&mut cur).unwrap()),
        "float" => Literal::Float(f64::read(&mut cur).unwrap()),
        "str" => Literal::String(String::read(&mut cur).unwrap()),
        "char" => Literal::Char(char::read(&mut cur).unwrap()),
        "bool" => Literal::Bool(bool::read(&mut cur).unwrap()),
        "typename" => Literal::TypeName(String::read(&mut cur).unwrap()),
        "void" => Literal::Void,
        _ => Literal::Struct(Box::new(StructureInstance::read(&mut cur).unwrap()))
    }
}

#[doc(hidden)]
pub fn __mem_feature<V>(visitor: &mut V) where V: Visitor {
    extern_fns!(visitor {
        scope "std::mem" {
            extern fn transmute(value, ty) -> unknown;
        }
    })
}