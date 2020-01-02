pub use crate::bytecode::{ArithOp, Cond, Op, PopMode};
use std::collections::HashMap;

pub trait Disass {
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
                writeln!(out, "section .const").unwrap();

                for v in vs {
                    writeln!(out, "{}", v).unwrap();
                }
            }
            Section::Code(stmts) => {
                writeln!(out, "section .code").unwrap();
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
    PushStack(i64),
    Jmp(Cond, String),
    Arith(ArithOp),
    Output(i64),
    Pop(i64),
    Label(String),
    Noop,
}
impl Disass for Stmt {
    fn print_lines(&self, out: &mut dyn std::io::Write) {
        match self {
            Stmt::PushInline(v) => writeln!(out, "    push {}", v),
            Stmt::PushConst(v) => writeln!(out, "    push const.{}", v),
            Stmt::PushStack(v) => writeln!(out, "    push stack.{}", v),
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
            Stmt::Arith(ArithOp::Or) => writeln!(out, "    or"),
            Stmt::Arith(ArithOp::And) => writeln!(out, "    and"),
            Stmt::Arith(ArithOp::Equal) => writeln!(out, "    eq"),
            Stmt::Arith(ArithOp::NotEqual) => writeln!(out, "    neq"),
            Stmt::Arith(ArithOp::LessThan) => writeln!(out, "    lt"),
            Stmt::Arith(ArithOp::LessEqual) => writeln!(out, "    le"),
            Stmt::Output(channel) => writeln!(out, "    output #{}", channel),
            Stmt::Pop(num) if *num == 1 => writeln!(out, "    pop"),
            Stmt::Pop(num) => writeln!(out, "    pop {}", num),
            Stmt::Noop => writeln!(out, "    noop"),
            Stmt::Label(label) => writeln!(out, "{}:", label),
        }
        .unwrap();
    }
}

pub trait BytecodeEmit {
    fn num_ops(&self) -> usize;
    fn emit(&self, labels: &HashMap<String, usize>, consts: &Vec<i64>, out: &mut Vec<Op>);
}

impl BytecodeEmit for Stmt {
    fn num_ops(&self) -> usize {
        match self {
            Stmt::Label(_) => 0,
            Stmt::PushInline(_)
            | Stmt::PushConst(_)
            | Stmt::PushStack(_)
            | Stmt::Arith(_)
            | Stmt::Output(_)
            | Stmt::Noop => 1,
            Stmt::Pop(n) if *n == 1 => 1,
            Stmt::Jmp(_, _) | Stmt::Pop(_) => 2,
        }
    }
    fn emit(&self, labels: &HashMap<String, usize>, consts: &Vec<i64>, out: &mut Vec<Op>) {
        match self {
            Stmt::PushInline(v) if *v <= 0x7FFF => out.push(Op::PushImmediate(*v as i16)),
            Stmt::PushInline(v) if *v <= 0xFFFFFF => {
                out.push(Op::PushImmediate24((*v as u32).into()))
            }
            Stmt::PushInline(v) => {
                let i = consts
                    .iter()
                    .position(|x| x == v)
                    .expect("missing const for large value");
                out.push(Op::PushConst(i as u16))
            }
            Stmt::PushConst(i) => out.push(Op::PushConst(*i as u16)),
            Stmt::PushStack(i) => out.push(Op::PushStack(*i as i16)),
            Stmt::Jmp(cond, label) => {
                let rel_addr = *labels.get(label).unwrap() as i64 - out.len() as i64;
                out.push(Op::PushImmediate((rel_addr - 1) as i16));
                out.push(Op::Jmp(cond.clone()));
            }
            Stmt::Arith(op) => out.push(Op::Arith(op.clone())),
            Stmt::Output(channel) => out.push(Op::Output(*channel as u16)),
            Stmt::Pop(n) if *n == 1 => out.push(Op::Pop(PopMode::One)),
            Stmt::Pop(n) => {
                if *n > 0x7FFF {
                    panic!("TODO: pop n > 0x7FFF not implemented"); // support 24bit / const push
                }
                out.push(Op::PushImmediate(*n as i16));
                out.push(Op::Pop(PopMode::Top));
            }
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

pub fn extract_constants(stmts: &Vec<Stmt>, c: &mut Vec<i64>) {
    for stmt in stmts {
        match stmt {
            Stmt::PushInline(v) if *v > 0xFFFFFF => {
                if !c.contains(v) {
                    c.push(v.clone());
                }
            }
            _ => (),
        }
    }
}

lalrpop_mod!(pub xas);
#[test]
fn asm_basic() {
    let program = xas::ProgramParser::new()
        .parse(include_str!("test_basic.xas"))
        .unwrap();

    println!("{:?}", program);
    for section in &program {
        section.print_lines(&mut std::io::stdout().lock());
    }
    if let (Section::Data(data), Section::Code(stmts)) = (&program[0], &program[1]) {
        let mut data = data.clone();
        let labels = label_locations(stmts);
        extract_constants(stmts, &mut data);
        println!("labels: {:?}", labels);
        let mut bc = Vec::new();

        for stmt in stmts {
            stmt.emit(&labels, &data, &mut bc);
        }
        bc.push(Op::Noop);
        println!("bc: {:?}", bc);
    }
}
