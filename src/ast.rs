use handy::{Handle, HandleMap};
use std::fmt::{Debug, Error, Formatter};

#[derive(Debug)]
pub enum Stmt {
    LetBinding(Ident, Expr),
    Print(Vec<Expr>),
    IfElse(Expr, Box<Stmt>, Option<Box<Stmt>>),
    Block(Vec<Stmt>),
}

pub trait HandleMapDedup<T: Eq> {
    fn get_dedup(&mut self, item: T) -> Handle;
}

impl<T: Eq> HandleMapDedup<T> for HandleMap<T> {
    fn get_dedup(&mut self, item: T) -> Handle {
        if let Some(h) = self.find_handle(&item) {
            h
        } else {
            self.insert(item)
        }
    }
}

pub type Ident = Handle;

// #[derive(Debug)]
pub enum Expr {
    Number(i64),
    EnvLoad(Ident),
    Op(Box<Expr>, Opcode, Box<Expr>),
    Error,
}

#[derive(Copy, Clone)]
pub enum Opcode {
    Mul,
    Div,
    Add,
    Sub,
    Or,
    And,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
}

impl Debug for Expr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        use self::Expr::*;
        match *self {
            Number(n) => write!(fmt, "{:?}", n),
            Op(ref l, op, ref r) => write!(fmt, "({:?} {:?} {:?})", l, op, r),
            EnvLoad(ident) => write!(fmt, "load({:?})", ident),
            Error => write!(fmt, "error"),
        }
    }
}

impl Debug for Opcode {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        use self::Opcode::*;
        match *self {
            Mul => write!(fmt, "*"),
            Div => write!(fmt, "/"),
            Add => write!(fmt, "+"),
            Sub => write!(fmt, "-"),
            Or => write!(fmt, "or"),
            And => write!(fmt, "and"),
            Equal => write!(fmt, "=="),
            NotEqual => write!(fmt, "!="),
            LessThan => write!(fmt, "<"),
            LessEqual => write!(fmt, "<="),
            GreaterThan => write!(fmt, ">"),
            GreaterEqual => write!(fmt, ">="),
        }
    }
}

#[test]
fn test_handy() {
    let mut map = HandleMap::new();
    let x = "hello hello blub";
    let h1 = map.get_dedup(&x[0..5]);
    let h2 = map.get_dedup(&x[6..11]);
    let h3 = map.get_dedup(&x[12..16]);
    assert_eq!(h1, h2);
    assert!(h1 != h3);
    assert_eq!(map.get(h1).unwrap(), &"hello");
    assert_eq!(map.get(h3).unwrap(), &"blub");
}
