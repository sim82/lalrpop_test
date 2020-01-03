pub use crate::bytecode::{ArithOp, Cond, Op, PopMode};
use log::debug;
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
    Move(i64),
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
            Stmt::Move(offs) => writeln!(out, "    move {}", offs),
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
            Stmt::Arith(_) | Stmt::Output(_) | Stmt::Noop => 1,
            Stmt::PushInline(n) if *n <= 0xFFFFFF => 1,
            Stmt::PushInline(_) => 2,
            Stmt::Pop(n) if *n == 0 => 0,
            Stmt::Pop(n) if *n == 1 => 1,
            Stmt::Jmp(_, _) | Stmt::Pop(_) => 2,
            Stmt::Move(_) | Stmt::PushConst(_) | Stmt::PushStack(_) => 2,
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
                if i > 0x7FFF {
                    panic!("TODO: pop n > 0x7FFF not implemented"); // support 24bit
                }
                out.push(Op::PushImmediate(i as i16));
                out.push(Op::PushConst);
            }
            Stmt::PushConst(i) => {
                if *i > 0x7FFF {
                    panic!("TODO: pop n > 0x7FFF not implemented"); // support 24bit
                }
                out.push(Op::PushImmediate(*i as i16));
                out.push(Op::PushConst);
            }
            Stmt::PushStack(i) => {
                if *i > 0x7FFF {
                    panic!("TODO: pop n > 0x7FFF not implemented"); // support 24bit
                }
                out.push(Op::PushImmediate(*i as i16));
                out.push(Op::PushStack);
            }
            Stmt::Jmp(cond, label) => {
                let rel_addr = *labels.get(label).unwrap() as i64 - out.len() as i64;
                out.push(Op::PushImmediate((rel_addr - 1) as i16));
                out.push(Op::Jmp(cond.clone()));
            }
            Stmt::Arith(op) => out.push(Op::Arith(op.clone())),
            Stmt::Output(channel) => out.push(Op::Output(*channel as u16)),
            Stmt::Pop(n) if *n == 0 => (), // the compiler will just stupidly emit 'pop 0' in some cases
            Stmt::Pop(n) if *n == 1 => out.push(Op::Pop(PopMode::One)),
            Stmt::Pop(n) => {
                if *n > 0x7FFF {
                    panic!("TODO: pop n > 0x7FFF not implemented"); // support 24bit / const push
                }
                out.push(Op::PushImmediate(*n as i16));
                out.push(Op::Pop(PopMode::Top));
            }
            Stmt::Move(offs) => {
                if *offs > 0x7FFF {
                    panic!("TODO: pop n > 0x7FFF not implemented"); // support 24bit / const push
                }
                out.push(Op::PushImmediate(*offs as i16));
                out.push(Op::Move);
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
        debug!("loc: {} {:?}", ip, stmt);
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

#[test]
fn asm_labels() {
    let mut stmts = Vec::new();
    stmts.push(Stmt::Jmp(Cond::Always, "jmp_const".into()));
    stmts.push(Stmt::PushConst(0));
    stmts.push(Stmt::Label("jmp_const".into()));
    stmts.push(Stmt::PushInline(123));

    stmts.push(Stmt::Jmp(Cond::Always, "jmp_stack".into()));
    stmts.push(Stmt::PushStack(0));
    stmts.push(Stmt::Label("jmp_stack".into()));
    stmts.push(Stmt::PushInline(124));

    stmts.push(Stmt::Jmp(Cond::Always, "jmp_inline".into()));
    stmts.push(Stmt::PushInline(1));
    stmts.push(Stmt::Label("jmp_inline".into()));
    stmts.push(Stmt::PushInline(125));

    stmts.push(Stmt::Jmp(Cond::Always, "jmp_inline24".into()));
    stmts.push(Stmt::PushInline(0xFFFFF));
    stmts.push(Stmt::Label("jmp_inline24".into()));
    stmts.push(Stmt::PushInline(126));

    stmts.push(Stmt::Jmp(Cond::Always, "jmp_inline_?".into()));
    stmts.push(Stmt::PushInline(0xFFFFFFF));
    stmts.push(Stmt::Label("jmp_inline_?".into()));
    stmts.push(Stmt::PushInline(127));

    stmts.push(Stmt::Jmp(Cond::Always, "jmp_inline_large".into()));
    stmts.push(Stmt::PushInline(0xFFFFFFFFF));
    stmts.push(Stmt::Label("jmp_inline_large".into()));
    stmts.push(Stmt::PushInline(128));

    stmts.push(Stmt::Jmp(Cond::Always, "jmp_pop_0".into()));
    stmts.push(Stmt::Pop(0));
    stmts.push(Stmt::Label("jmp_pop_0".into()));
    stmts.push(Stmt::PushInline(129));

    stmts.push(Stmt::Jmp(Cond::Always, "jmp_pop_top".into()));
    stmts.push(Stmt::Pop(1));
    stmts.push(Stmt::Label("jmp_pop_top".into()));
    stmts.push(Stmt::PushInline(130));

    stmts.push(Stmt::Jmp(Cond::Always, "jmp_pop_large".into()));
    stmts.push(Stmt::Pop(0x7FFF));
    stmts.push(Stmt::Label("jmp_pop_large".into()));
    stmts.push(Stmt::PushInline(131));

    let mut data = Vec::new();
    let labels = label_locations(&stmts);

    extract_constants(&stmts, &mut data);
    // debug!("labels: {:?}", labels);
    let mut bc = Vec::new();

    for stmt in stmts {
        stmt.emit(&labels, &data, &mut bc);
    }
    bc.push(Op::Noop);
    assert_eq!(
        bc[*labels.get("jmp_pop_large").unwrap()],
        Op::PushImmediate(131)
    );
    assert_eq!(
        bc[*labels.get("jmp_pop_top").unwrap()],
        Op::PushImmediate(130)
    );
    assert_eq!(
        bc[*labels.get("jmp_pop_0").unwrap()],
        Op::PushImmediate(129)
    );
    assert_eq!(
        bc[*labels.get("jmp_inline_large").unwrap()],
        Op::PushImmediate(128)
    );
    assert_eq!(
        bc[*labels.get("jmp_inline24").unwrap()],
        Op::PushImmediate(126)
    );
    assert_eq!(
        bc[*labels.get("jmp_inline").unwrap()],
        Op::PushImmediate(125)
    );
    assert_eq!(
        bc[*labels.get("jmp_stack").unwrap()],
        Op::PushImmediate(124)
    );
    assert_eq!(
        bc[*labels.get("jmp_const").unwrap()],
        Op::PushImmediate(123)
    );
    // debug!("bc: {:?}", bc);
}
