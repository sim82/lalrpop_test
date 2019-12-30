use crate::ast::{Expr, Opcode, Stmt};
use handy::Handle;
use std::collections::HashMap;

pub struct Evaluator {
    // ident_env: &'input mut dyn HandleMapDedup<&'input str>,
    env: HashMap<Handle, i64>,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            // ident_env: &mut env,
            env: HashMap::new(),
        }
    }

    pub fn execute(&mut self, stmt: Stmt) {
        match stmt {
            Stmt::LetBinding(ident, expr) => {
                let v = self.eval(expr);
                // let h = self.ide
                self.env.insert(ident, v);
            }
            Stmt::Print(exprs) => {
                for e in exprs {
                    println!("Print: {}", self.eval(e));
                }
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.execute(s);
                }
            }
            Stmt::IfElse(e, if_stmt, else_stmt) => {
                let v = self.eval(e);
                println!("ifelse: {}", v);
                if v != 0 {
                    self.execute(*if_stmt);
                } else {
                    if let Some(else_stmt) = else_stmt {
                        self.execute(*else_stmt);
                    }
                }
            } // Stmt::Expr(e) => {
              //     self.eval(e);
              // }
        }
    }
    fn eval(&mut self, expr: Expr) -> i64 {
        match expr {
            Expr::Number(v) => v,
            Expr::EnvLoad(ident) => match self.env.get(&ident) {
                Some(v) => v.clone(),
                None => panic!("not in env: {:?}", ident),
            },
            Expr::Op(a, opcode, b) => match opcode {
                Opcode::Add => self.eval(*a) + self.eval(*b),
                Opcode::Sub => self.eval(*a) - self.eval(*b),
                Opcode::Mul => self.eval(*a) * self.eval(*b),
                Opcode::Div => self.eval(*a) / self.eval(*b),
            },
            Expr::Error => 666,
        }
    }
}
