use crate::{extern_fns, Parameters, unwrap_args};
use crate::tks::Literal;
use crate::visit::Visitor;

pub mod io;
pub mod math;
pub mod strs;

fn panic(params: Parameters) -> Literal {
    let msg = unwrap_args!(params => (String));
    eprintln!("Process panicked: {}", msg);
    std::process::exit(-1);
}

fn exit(params: Parameters) -> Literal {
    let exit_code = unwrap_args!(params => (Number));
    std::process::exit(exit_code as i32);
}

#[doc(hidden)]
pub fn __core_feature<V>(visitor: &mut V) where V: Visitor {
    extern_fns!(visitor {
        scope "std" {
            extern fn panic(message) -> void;
            extern fn exit(code) -> void;
        }
    })
}