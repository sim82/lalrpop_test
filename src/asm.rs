pub use crate::bytecode::{ArithOp, Cond, Op};
use std::collections::HashMap;

trait Disass {
    fn print_lines(&self, out: &mut dyn std::io::Write);
}

#[derive(Debug)]
pub enum Section {
    Data(Vec<i64>),
    Code(Vec<Stmt>),
}
impl Disass for Section {
    fn print_lines(&self, out: &mut dyn std::io::Write) {
        match self {
            Section::Data(vs) => {
                writeln!(out, "section .data");

                for v in vs {
                    writeln!(out, "{}", v);
                }
            }
            Section::Code(stmts) => {
                writeln!(out, "section .code");
                for s in stmts {
                    s.print_lines(out);
                }
            }
        }
    }
}
#[derive(Debug)]
pub enum Stmt {
    PushInline(i64),
    PushConst(i64),
    Jmp(Cond, String),
    Arith(ArithOp),
    Output(i64),
    Label(String),
    Noop,
}
impl Disass for Stmt {
    fn print_lines(&self, out: &mut dyn std::io::Write) {
        match self {
            Stmt::PushInline(v) => writeln!(out, "    push {}", v),
            Stmt::PushConst(v) => writeln!(out, "    push const.{}", v),
            Stmt::Jmp(cond, label) => {
                let cond = match cond {
                    Cond::Always => "always",
                    Cond::NonZero => "nz",
                    Cond::Zero => "z",
                };
                writeln!(out, "    jmp {} {}", cond, label)
            }
            Stmt::Arith(ArithOp::Add) => writeln!(out, "    add"),
            Stmt::Arith(ArithOp::Sub) => writeln!(out, "    sub"),
            Stmt::Arith(ArithOp::Mul) => writeln!(out, "    mul"),
            Stmt::Arith(ArithOp::Div) => writeln!(out, "    div"),
            Stmt::Output(channel) => writeln!(out, "    output #{}", channel),
            Stmt::Noop => writeln!(out, "    noop"),
            Stmt::Label(label) => writeln!(out, "{}:", label),
        }
        .unwrap();
    }
}

pub trait BytecodeEmit {
    fn num_ops(&self) -> usize;
    fn emit(&self, labels: &HashMap<String, usize>, out: &mut Vec<Op>);
}

impl BytecodeEmit for Stmt {
    fn num_ops(&self) -> usize {
        match self {
            Stmt::PushInline(_) => 1,
            Stmt::PushConst(_) => 1,
            Stmt::Jmp(_, _) => 2,
            Stmt::Arith(_) => 1,
            Stmt::Output(_) => 1,
            Stmt::Label(_) => 0,
            Stmt::Noop => 1,
        }
    }
    fn emit(&self, labels: &HashMap<String, usize>, out: &mut Vec<Op>) {
        match self {
            Stmt::PushInline(v) if *v <= 0x7FFF => out.push(Op::PushImmediate(*v as i16)),
            Stmt::PushInline(v) => out.push(Op::PushImmediate24((*v as u32).into())),
            Stmt::PushConst(i) => out.push(Op::PushConst(*i as u16)),
            Stmt::Jmp(cond, label) => {
                let rel_addr = labels.get(label).unwrap() - out.len();
                out.push(Op::PushImmediate((rel_addr - 1) as i16));
                out.push(Op::Jmp(cond.clone()));
            }
            Stmt::Arith(op) => out.push(Op::Arith(op.clone())),
            Stmt::Output(channel) => out.push(Op::Output(*channel as u16)),
            Stmt::Label(_) => (),
            Stmt::Noop => out.push(Op::Noop),
        }
    }
}

pub fn label_locations(stmts: &Vec<Stmt>) -> HashMap<String, usize> {
    let mut labels = HashMap::new();
    let mut ip = 0;
    for stmt in stmts {
        if let Stmt::Label(label) = stmt {
            labels.insert(label.clone(), ip);
        }
        ip += stmt.num_ops();
    }
    labels
}

lalrpop_mod!(pub xas);
#[test]
fn asm_basic() {
    let program = xas::ProgramParser::new()
        .parse(include_str!("test.xas"))
        .unwrap();

    println!("{:?}", program);
    for section in &program {
        section.print_lines(&mut std::io::stdout().lock());
    }
    if let Section::Code(stmts) = &program[1] {
        let labels = label_locations(stmts);
        println!("labels: {:?}", labels);
        let mut bc = Vec::new();

        for stmt in stmts {
            stmt.emit(&labels, &mut bc);
        }
        bc.push(Op::Noop);
        println!("bc: {:?}", bc);
    }
}
