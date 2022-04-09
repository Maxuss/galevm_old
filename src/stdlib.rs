use std::thread;
use std::time::Duration;
use crate::{extern_fns, Parameters, unwrap_args};
use crate::tks::Literal;
use crate::visit::Visitor;

pub mod io;
pub mod math;
pub mod strs;
pub mod mem;
pub mod prelude;

fn panic(params: Parameters) -> Literal {
    let msg = unwrap_args!(params => (String));
    eprintln!("Process panicked: {}", msg);
    std::process::exit(-1);
}

fn exit(params: Parameters) -> Literal {
    let exit_code = unwrap_args!(params => (Number));
    std::process::exit(exit_code as i32);
}

fn sleep(params: Parameters) -> Literal {
    let time = unwrap_args!(params => (Number));
    thread::sleep(Duration::from_secs(time as u64));
    Literal::Void
}

fn sleep_millis(params: Parameters) -> Literal {
    let time = unwrap_args!(params => (Number));
    thread::sleep(Duration::from_millis(time as u64));
    Literal::Void
}

#[doc(hidden)]
pub fn __core_feature<V>(visitor: &mut V) where V: Visitor {
    extern_fns!(visitor {
        scope "std" {
            extern fn panic(message) -> void;
            extern fn exit(code) -> void;
            extern fn sleep(time) -> void;
            extern fn sleep_millis(time) -> void;
        }
    })
}