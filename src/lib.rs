#[macro_use]
extern crate lalrpop_util;
// #[macro_use]
// extern crate log;

pub mod asm;
pub mod ast;
pub mod bytecode;
pub mod eval;
pub mod parser;

lalrpop_mod!(pub lang1);

#[cfg(test)]
mod test {
    use super::lang1;
    use crate::eval::Evaluator;
    use handy::HandleMap;
    //lalrpop_mod!(lang1);

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
        for s in expr.iter().filter_map(|x| match x {
            crate::ast::Toplevel::Stmt(s) => Some(s),
            _ => None,
        }) {
            evaluator.execute(s.clone());
        }

        let expr = lang1::ProgramParser::new()
            .parse(
                &mut env,
                &mut errors,
                "let a = 41 + 1; if a {print 10 * 10;} else {print 123 * 0b101, a;}",
            )
            .unwrap();
        for s in expr.iter().filter_map(|x| match x {
            crate::ast::Toplevel::Stmt(s) => Some(s),
            _ => None,
        }) {
            println!("execute: {:?}", s);
            evaluator.execute(s.clone());
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
}
