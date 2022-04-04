#![feature(fn_traits)]
#![feature(box_patterns)]

extern crate core;

use anyhow::bail;

pub mod builtin;
pub mod fns;
pub mod structs;
pub mod tks;
pub mod var;
pub mod visit;
pub mod vm;

pub trait ToResult<T> {
    fn to_result(&self) -> anyhow::Result<T>;
}

impl<T> ToResult<T> for Option<T>
where
    T: Clone,
{
    fn to_result(&self) -> anyhow::Result<T> {
        if self.is_some() {
            let unwrap = self.as_ref().unwrap();
            Ok(unwrap.to_owned())
        } else {
            bail!("Empty option!")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::structs::Structure;
    use crate::tks::{BinaryOp, Expression, Keyword, Literal, Token};
    use crate::visit::{Visitor, Vm};
    use std::time::Instant;

    #[test]
    fn test_exprs() {
        let mut vm = Vm::new();
        // Short overview of token chain:
        // Expression { Add(200, 300) }
        // Literal { Ident("constant") }
        // Const
        //
        // Literal { String("Hello, World!") }
        // Literal { Ident("mutable_var") }
        // Let
        //
        // In pseudocode it can be written as
        // const constant = 200 + 300;
        // let mutable_var = "Hello, World!";
        let mut chain = vec![
            Token::Keyword(Keyword::Const),
            Token::Literal(Literal::Ident(String::from("constant"))),
            Token::Expression(Box::new(Expression::BinaryOp(
                BinaryOp::Add,
                Token::Literal(Literal::Number(200)),
                Token::Literal(Literal::Number(300)),
            ))),
            Token::Keyword(Keyword::Let),
            Token::Literal(Literal::Ident(String::from("mutable_var"))),
            Token::Literal(Literal::String(String::from("Hello, World!"))),
        ];
        vm.load_chain(&mut chain);
        vm.process();
        println!("{:#?}", vm);
    }

    #[test]
    fn test_structs() {
        let mut vm = Vm::new();
        let mut str = Structure::with_type("Structure");
        str.add_var("cool_var".to_string(), Literal::Bool(true));
        str.add_const("cool_const".to_string(), Literal::Number(1200));
        let mut chain = vec![Token::Expression(Box::new(Expression::InvokeBuiltin(
            "debugp".to_string(),
            vec![Token::Literal(Literal::Struct(Box::new(str)))],
        )))];
        vm.load_chain(&mut chain);
    }

    #[test]
    fn test_functions() {
        let mut vm = Vm::new();
        let mut chain = vec![
            Token::Keyword(Keyword::Function),
            Token::Literal(Literal::TypeName("void".to_string())),
            Token::Literal(Literal::Ident("say_hello".to_string())),
            Token::LParen,
            Token::Literal(Literal::Ident("name".to_string())),
            Token::RParen,
            Token::LBracket,
            Token::Expression(Box::new(Expression::InvokeBuiltin(
                "fmt".to_string(),
                vec![
                    Token::Literal(Literal::String("Hello, ".to_string())),
                    Token::Literal(Literal::Ident("name".to_string())),
                ],
            ))),
            Token::Literal(Literal::Ident("greeting".to_string())),
            Token::Keyword(Keyword::Let),
            Token::Expression(Box::new(Expression::InvokeBuiltin(
                "println".to_string(),
                vec![Token::Literal(Literal::Ident("greeting".to_string()))],
            ))),
            Token::Keyword(Keyword::Return),
            Token::Literal(Literal::Void),
            Token::RBracket,
            Token::Expression(Box::new(Expression::InvokeStatic(
                "global::say_hello".to_string(),
                vec![Token::Literal(Literal::String("World!".to_string()))],
            ))),
        ];
        let start = Instant::now();
        vm.load_chain(&mut chain);
        vm.process();
        let dur = Instant::now() - start;
        println!("Finished in {} mcs", dur.as_micros())
    }

    #[test]
    fn test_if_else_elif() {
        let mut vm = Vm::new();
        let mut chain = vec![
            Token::Expression(Box::new(Expression::IfStmt)),
            Token::Literal(Literal::Bool(false)),
            Token::LBracket,
            Token::Expression(Box::new(Expression::InvokeBuiltin(
                "println".to_string(),
                vec![Token::Literal(Literal::String("If executed!".to_string()))],
            ))),
            Token::RBracket,
            Token::Expression(Box::new(Expression::ElifStmt)),
            Token::Literal(Literal::Bool(true)),
            Token::LBracket,
            Token::Expression(Box::new(Expression::InvokeBuiltin(
                "println".to_string(),
                vec![Token::Literal(Literal::String(
                    "Elif executed!".to_string(),
                ))],
            ))),
            Token::RBracket,
            Token::Expression(Box::new(Expression::ElseStmt)),
            Token::LBracket,
            Token::Expression(Box::new(Expression::InvokeBuiltin(
                "println".to_string(),
                vec![Token::Literal(Literal::String(
                    "Else executed!".to_string(),
                ))],
            ))),
            Token::RBracket,
        ];
        let start = Instant::now();
        vm.load_chain(&mut chain);
        vm.process();
        let dur = Instant::now() - start;
        println!("Finished in {} mcs", dur.as_micros())
    }

    #[test]
    fn test_while() {
        let mut vm = Vm::new();
        let mut chain = vec![
            Token::Keyword(Keyword::Let),
            Token::Literal(Literal::Ident("i".to_string())),
            Token::Literal(Literal::Number(0)),
            Token::Expression(Box::new(Expression::WhileStmt)),
            Token::Expression(Box::new(Expression::BinaryOp(
                BinaryOp::Lt,
                Token::Literal(Literal::Ident("i".to_string())),
                Token::Literal(Literal::Number(10)),
            ))),
            Token::LBracket,
            Token::Expression(Box::new(Expression::BinaryOp(
                BinaryOp::Assign,
                Token::Literal(Literal::Ident("i".to_string())),
                Token::Expression(Box::new(Expression::BinaryOp(
                    BinaryOp::Add,
                    Token::Literal(Literal::Ident("i".to_string())),
                    Token::Literal(Literal::Number(1)),
                ))),
            ))),
            Token::Expression(Box::new(Expression::InvokeBuiltin(
                "debug".to_string(),
                vec![Token::Literal(Literal::Ident("i".to_string()))],
            ))),
            Token::RBracket,
        ];
        let start = Instant::now();
        vm.load_chain(&mut chain);
        vm.process();
        let dur = Instant::now() - start;
        println!("Finished in {} mcs", dur.as_micros())
    }

    #[test]
    fn test_transmute() {
        let mut vm = Vm::new();
        let mut chain = vec![
            Token::Keyword(Keyword::Let),
            Token::Literal(Literal::Ident("first".to_string())),
            Token::Literal(Literal::Number(120000)),
            Token::Keyword(Keyword::Let),
            Token::Literal(Literal::Ident("second".to_string())),
            Token::Expression(Box::new(Expression::InvokeBuiltin(
                "transmute".to_string(),
                vec![
                    Token::Literal(Literal::Ident("first".to_string())),
                    Token::Literal(Literal::TypeName("float".to_string())),
                ],
            ))),
            Token::Expression(Box::new(Expression::InvokeBuiltin(
                "debug".to_string(),
                vec![Token::Literal(Literal::Ident("first".to_string()))],
            ))),
            Token::Expression(Box::new(Expression::InvokeBuiltin(
                "debug".to_string(),
                vec![Token::Literal(Literal::Ident("second".to_string()))],
            ))),
        ];
        let start = Instant::now();
        vm.load_chain(&mut chain);
        vm.process();
        let dur = Instant::now() - start;
        println!("Finished in {} mcs", dur.as_micros())
    }
}
