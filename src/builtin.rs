use crate::fns::Parameters;
use crate::tks::Literal;

pub fn find_builtin(name: String) -> impl Fn(Parameters) -> Literal {
    match name.as_str() {
        "panic" => _panic,
        "debug" => _debug,
        "debugp" => _pretty_debug,
        "fmt" => _fmt,
        "print" => _print,
        "println" => _println,
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
