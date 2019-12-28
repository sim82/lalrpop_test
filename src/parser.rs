use crate::ast::{Expr, Opcode};

pub fn binop(a: Expr, op: Opcode, b: Expr) -> Expr {
    Expr::Op(Box::new(a), op, Box::new(b))
}
