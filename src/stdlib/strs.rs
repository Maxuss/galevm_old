use crate::{extern_fns, Parameters};
use crate::tks::Literal;
use crate::visit::Visitor;

fn stringify(params: Parameters) -> Literal {
    Literal::String(params.get(0).unwrap().to_string())
}

#[doc(hidden)]
pub fn __str_feature<V>(visitor: &mut V) where V: Visitor {
    extern_fns!(visitor {
        scope "std::str" {
            extern fn stringify(value) -> str;
        }
    })
}