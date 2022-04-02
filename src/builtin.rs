use std::io::Cursor;
use crate::fns::Parameters;
use crate::structs::Structure;
use crate::tks::Literal;
use crate::vm::Transmute;

pub fn find_builtin(name: String) -> impl Fn(Parameters) -> Literal {
    match name.as_str() {
        "panic" => _panic,
        "debug" => _debug,
        "debugp" => _pretty_debug,
        "fmt" => _fmt,
        "print" => _print,
        "println" => _println,
        "transmute" => _transmute,
        _ => {
            panic!("Tried to call unknown builtin function {}!", name)
        }
    }
}

fn _print(params: Parameters) -> Literal {
    if let Literal::String(str) = params.get(0).unwrap() {
        print!("{}", str);
    } else {
        print!("EOF");
    };
    Literal::Void
}

fn _println(params: Parameters) -> Literal {
    if let Literal::String(str) = params.get(0).unwrap() {
        println!("{}", str);
    } else {
        println!("EOF");
    };
    Literal::Void
}

fn _debug(params: Parameters) -> Literal {
    let mut str = String::new();
    for param in params {
        str.push_str(&format!(": {:?}\n", param))
    }

    println!("{}", str);
    Literal::Void
}

fn _pretty_debug(params: Parameters) -> Literal {
    let mut str = String::new();
    for param in params {
        str.push_str(&format!(": {:#?}\n", param))
    }
    println!("{}", str);
    Literal::Void
}

fn _fmt(params: Parameters) -> Literal {
    let mut str = String::new();
    for param in params {
        str.push_str(&format!("{}", param))
    }

    Literal::String(str)
}

fn _panic(params: Parameters) -> Literal {
    if let Literal::String(msg) = params.get(0).unwrap() {
        panic!("Panic! {}", msg)
    };
    Literal::Void
}

fn _transmute(params: Parameters) -> Literal {
    let mut value = params.get(0).unwrap().to_owned();
    if let Literal::TypeName(typename) = params.get(1).unwrap() {
        let mut staging: Vec<u8> = Vec::new();
        value.write(&mut staging).unwrap();
        let mut cursor = Cursor::new(staging);
        match typename.as_str() {
            "num" => Literal::Number(i64::read(&mut cursor).unwrap()),
            "float" => Literal::Float(f64::read(&mut cursor).unwrap()),
            "str" => Literal::String(String::read(&mut cursor).unwrap()),
            "char" => Literal::Char(char::read(&mut cursor).unwrap()),
            "bool" => Literal::Bool(bool::read(&mut cursor).unwrap()),
            "typename" => Literal::TypeName(String::read(&mut cursor).unwrap()),
            "void" => Literal::Void,
            _ => Literal::Struct(Box::new(Structure::read(&mut cursor).unwrap()))
        }
    } else {
        panic!("Expected a typename to be transmuted!")
    }
}