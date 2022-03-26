pub mod tks;
pub mod visit;
pub mod vm;
pub mod var;

#[cfg(test)]
mod tests {
    use crate::tks::{BinaryOp, Expression, Keyword, Literal, Token};
    use crate::visit::{Visitor, Vm};
    use crate::vm::Memory;

    #[test]
    fn test_ptrs() -> anyhow::Result<()> {
        let mut vm = Vm::new();
        let ptr = vm.alloc(8)?; // allocating size of u64
        vm.write(ptr, &mut 120000u64)?; // writing the value
        let value: u64 = vm.read_const(ptr)?; // reading the value
        assert_eq!(value, 120000);
        vm.free(ptr, 8)?; // freeing memory from vm
        Ok(())
    }

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
            Token::Expression(Box::new(
                Expression::BinaryOp(
                    BinaryOp::Add,
                    Token::Literal(Literal::Number(200)),
                    Token::Literal(Literal::Number(300)))
            )),
            Token::Literal(Literal::Ident(String::from("constant"))),
            Token::Keyword(Keyword::Const),
            Token::Literal(Literal::String(String::from("Hello, World!"))),
            Token::Literal(Literal::Ident(String::from("mutable_var"))),
            Token::Keyword(Keyword::Let)
        ];
        vm.process_chain(&mut chain);
        vm.free(0, 23).unwrap();
        println!("{:#?}", vm);
    }
}