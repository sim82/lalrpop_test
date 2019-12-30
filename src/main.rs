#[macro_use]
extern crate lalrpop_util;
pub mod ast;
pub mod bytecode;
pub mod eval;
pub mod parser;

use crate::{
    ast::{HandleMapDedup, Opcode, Stmt},
    eval::Evaluator,
};
use handy::HandleMap;
use std::collections::HashMap;

fn main() {
    println!("Hello, world!");
}

lalrpop_mod!(pub lang1);

#[test]
fn test1() {
    let mut env = HandleMap::new();
    let mut errors = Vec::new();

    let expr = lang1::ProgramParser::new()
        .parse(
            &mut env,
            &mut errors,
            "let a = 41 + 1; print 10 * 10, 123 * 0b101, a;",
        )
        .unwrap();

    println!("{:?}", expr);
    println!("env: {:?}", env);
    let mut evaluator = Evaluator::new();
    for s in expr {
        evaluator.execute(s);
    }

    let expr = lang1::ProgramParser::new()
        .parse(
            &mut env,
            &mut errors,
            "let a = 41 + 1; if a {print 10 * 10;} else {print 123 * 0b101, a;}",
        )
        .unwrap();
    for s in expr {
        println!("execute: {:?}", s);
        evaluator.execute(s);
    }
}

#[test]
fn lang1_errors() {
    let mut env = HandleMap::new();
    let mut errors = Vec::new();

    let expr = lang1::ExprsParser::new()
        .parse(&mut env, &mut errors, "22 * + 3")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * error) + 3)]");

    let expr = lang1::ExprsParser::new()
        .parse(&mut env, &mut errors, "22 * 44 + 66, *3")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * 44) + 66), (error * 3)]");

    let expr = lang1::ExprsParser::new()
        .parse(&mut env, &mut errors, "*")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[(error * error)]");

    assert_eq!(errors.len(), 4);
}
