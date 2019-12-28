#[macro_use]
extern crate lalrpop_util;
pub mod ast;
pub mod parser;

use crate::ast::{Expr, Opcode, Stmt};

fn main() {
    println!("Hello, world!");
}

lalrpop_mod!(pub lang1);

fn eval(expr: Expr) -> i64 {
    match expr {
        Expr::Number(v) => v,
        Expr::Op(a, opcode, b) => match opcode {
            Opcode::Add => eval(*a) + eval(*b),
            Opcode::Sub => eval(*a) - eval(*b),
            Opcode::Mul => eval(*a) * eval(*b),
            Opcode::Div => eval(*a) / eval(*b),
        },
        Expr::Error => 666,
    }
}

#[test]
fn lang1() {
    let mut errors = Vec::new();

    let expr = lang1::ExprsParser::new()
        .parse(&mut errors, "22 * + 3")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * error) + 3)]");

    let expr = lang1::ExprsParser::new()
        .parse(&mut errors, "22 * 44 + 66, *3")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * 44) + 66), (error * 3)]");

    let expr = lang1::ExprsParser::new().parse(&mut errors, "*").unwrap();
    assert_eq!(&format!("{:?}", expr), "[(error * error)]");

    assert_eq!(errors.len(), 4);

    let expr = lang1::ProgramParser::new()
        .parse(
            &mut errors,
            "let a = 123 * 0b101; print 10 * 10, 123 * 0b101;",
        )
        .unwrap();

    println!("{:?}", expr);

    for s in expr {
        match s {
            Stmt::LetBinding(_, _) => println!("let binding"),
            Stmt::Print(exprs) => {
                for e in exprs {
                    println!("eval: {}", eval(e))
                }
            }
        }
    }
}
