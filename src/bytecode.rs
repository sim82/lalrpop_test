use log::debug;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub struct Uint24([u8; 3]);

impl From<u32> for Uint24 {
    fn from(v: u32) -> Self {
        Self([
            ((v >> 16) & 0xFF) as u8,
            ((v >> 8) & 0xFF) as u8,
            (v & 0xFF) as u8,
        ])
    }
}

impl Into<u32> for Uint24 {
    fn into(self) -> u32 {
        ((self.0[0] as u32) << 16) | ((self.0[1] as u32) << 8) | self.0[2] as u32
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Or,
    And,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
}

fn bool_to_i64(v: bool) -> i64 {
    if v {
        1
    } else {
        0
    }
}

impl ArithOp {
    pub fn eval(&self, a: i64, b: i64) -> i64 {
        match *self {
            ArithOp::Add => a + b,
            ArithOp::Sub => a - b,
            ArithOp::Mul => a * b,
            ArithOp::Div => a / b,
            ArithOp::Or => bool_to_i64(a != 0 || b != 0),
            ArithOp::And => bool_to_i64(a != 0 && b != 0),
            ArithOp::Equal => bool_to_i64(a == b),
            ArithOp::NotEqual => bool_to_i64(a != b),
            ArithOp::LessThan => bool_to_i64(a < b),
            ArithOp::LessEqual => bool_to_i64(a <= b),
        }
    }
}

// #[derive(Clone, Serialize, Deserialize, Debug, Copy)]
// enum JmpType {
//     Abs,
//     Rel,
// }

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum PopMode {
    One,
    Top,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum Cond {
    Always,
    Zero,
    NonZero,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum Op {
    Noop,
    PushConst(u16),
    PushStack(i16),
    PushImmediate(i16),
    PushImmediate24(Uint24),
    Move,
    Arith(ArithOp),
    Jmp(Cond),
    Output(u16),
    Pop(PopMode),
    Break,
}

#[derive(Serialize, Deserialize)]
pub struct Program {
    pub data: Vec<i64>,
    pub code: Vec<Op>,
}
impl Program {
    pub fn new() -> Self {
        Program {
            data: Vec::new(),
            code: Vec::new(),
        }
    }
}

pub struct Vm {
    pub data: Vec<i64>,
    stack: Vec<i64>,
    pub code: Vec<Op>,
    ip: usize,
    pub num_ops: usize,
    pub max_ops: Option<usize>,
}
pub struct IoChannels {
    pub channels: Vec<Sender<i64>>,
}
impl IoChannels {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
        }
    }
}
impl Vm {
    pub fn new() -> Self {
        Vm {
            data: Vec::new(),
            stack: Vec::new(),
            code: Vec::new(),
            ip: 0,
            num_ops: 0,
            max_ops: None,
        }
    }
    pub fn from_program(prog: Program) -> Self {
        Vm {
            data: prog.data,
            stack: Vec::new(),
            code: prog.code,
            ip: 0,
            num_ops: 0,
            max_ops: None,
        }
    }
    pub fn push(&mut self, v: i64) {
        self.stack.push(v);
    }
    pub fn pop(&mut self) -> i64 {
        self.stack.pop().unwrap()
    }
    pub fn peek(&self) -> i64 {
        *self.stack.last().unwrap()
    }
    pub fn peek_at(&self, offs: i64) -> i64 {
        if offs >= 0 && (offs as usize) < self.stack.len() {
            return *self
                .stack
                .get(self.stack.len() - 1 - offs as usize)
                .unwrap();
        }
        panic!("stack underflow: {} (of {})", offs, self.stack.len());
    }
    pub fn peek_at_mut(&mut self, offs: i64) -> &mut i64 {
        if offs >= 0 && (offs as usize) < self.stack.len() {
            let top = self.stack.len() - 1;
            return self.stack.get_mut(top - offs as usize).unwrap();
        }
        panic!("stack underflow: {} (of {})", offs, self.stack.len());
    }
    pub fn exec(&mut self, io: Option<&IoChannels>) {
        while self.ip < self.code.len() {
            let op = self.code[self.ip].clone();
            self.num_ops += 1;
            if let Some(max_ops) = self.max_ops {
                if self.num_ops > max_ops {
                    debug!("max ops reached: {}", max_ops);
                    break;
                }
            }
            debug!("exec: {} {:?}", self.ip, op);
            match op {
                Op::PushConst(offs) => {
                    self.push(self.data[offs as usize]);
                }
                Op::PushStack(offs) => self.push(self.peek_at(offs as i64)),
                Op::Arith(op) => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(op.eval(a, b));
                }
                Op::PushImmediate(v) => self.push(v as i64),
                Op::PushImmediate24(v) => {
                    let v: u32 = v.into();
                    self.push(v as i64)
                }
                Op::Jmp(jmp_cond) => {
                    let dst = self.pop();
                    let cond = match jmp_cond {
                        Cond::Always => true,
                        Cond::Zero => self.pop() == 0,
                        Cond::NonZero => self.pop() != 0,
                    };
                    debug!("jmp: {} {}", cond, dst);

                    if cond {
                        self.ip = (self.ip as i64 + dst) as usize;
                        if self.ip >= self.code.len() {
                            panic!(
                                "jmp to invalid code location {} (of {})",
                                self.ip,
                                self.code.len()
                            );
                        }
                        continue;
                    }
                }
                Op::Output(channel) => {
                    let v = self.pop();
                    if let Some(io) = &io {
                        debug!("output #{}: {}", channel, v);
                        io.channels[channel as usize].send(v).unwrap();
                    }
                }
                Op::Pop(PopMode::One) => {
                    self.pop();
                }
                Op::Pop(PopMode::Top) => {
                    let n = self.pop();
                    for _ in 0..n {
                        self.pop();
                    }
                }
                Op::Move => {
                    let offs = self.pop();
                    let v = self.pop();
                    *self.peek_at_mut(offs) = v;
                }
                Op::Noop => (),
                Op::Break => {
                    break;
                }
            }
            self.ip += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use log::info;

    use super::*;
    use std::sync::mpsc::channel;
    #[test]
    fn arith() {
        let mut prog = Program::new();
        prog.data.push(123);
        prog.data.push(666);
        prog.data.push(777);

        prog.code.push(Op::PushConst(0));
        prog.code.push(Op::PushConst(1));
        // prog.code.push(Op::Add);
        prog.code.push(Op::Arith(ArithOp::Add));

        prog.code.push(Op::PushConst(1));
        // prog.code.push(Op::PushConst(2));
        // prog.code.push(Op::PushImmediate24(0xaabbcc.into()));
        prog.code.push(Op::PushImmediate(1666));

        // prog.code.push(Op::Sub);
        prog.code.push(Op::Arith(ArithOp::Sub));
        prog.code.push(Op::Output(0));
        prog.code.push(Op::Output(0));
        println!("{}", serde_yaml::to_string(&prog).unwrap());
        let bin = bincode::serialize(&prog.code).unwrap();
        println!("{} {:?}", bin.len(), bin);
        let (sender, receiver) = channel();
        let mut io = IoChannels::new();
        io.channels.push(sender);
        let mut vm = Vm::from_program(prog);
        vm.exec(Some(&io));
        println!(
            "sub: {} add: {}",
            receiver.recv().unwrap(),
            receiver.recv().unwrap()
        );
        // println!("sub: {}", vm.pop());
        // println!("add: {}", vm.pop());

        println!("{}", std::mem::size_of::<Op>());

        assert_eq!(std::mem::size_of::<Op>(), std::mem::size_of::<u32>());
        unsafe {
            let mut y = [Op::Arith(ArithOp::Add); 8]; // = vm.code[0..6];
                                                      // std::slice::bytes::copy_memory(&vm.code, &mut y);
                                                      // let x = std::mem::transmute::<[Op; 6], [u8; 6 * 4]>(y);
            y.clone_from_slice(&vm.code[..]);
            let z: [u8; 8 * 4] = std::mem::transmute_copy(&y);
            println!("z: {:?}", z);
        }
    }

    #[test]
    fn arith_ops() {
        let mut prog = Program::new();

        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::Arith(ArithOp::Add));

        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::Arith(ArithOp::Sub));

        prog.code.push(Op::PushImmediate(3));
        prog.code.push(Op::PushImmediate(4));
        prog.code.push(Op::Arith(ArithOp::Mul));

        prog.code.push(Op::PushImmediate(6));
        prog.code.push(Op::PushImmediate(3));
        prog.code.push(Op::Arith(ArithOp::Div));

        let mut vm = Vm::from_program(prog);
        vm.exec(None);

        assert_eq!(vm.pop(), 2);
        assert_eq!(vm.pop(), 12);
        assert_eq!(vm.pop(), 1);
        assert_eq!(vm.pop(), 3);
    }
    #[test]
    fn arith_eq() {
        let mut prog = Program::new();

        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::PushImmediate(3));
        prog.code.push(Op::Arith(ArithOp::Equal));

        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::Arith(ArithOp::Equal));

        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::PushImmediate(3));
        prog.code.push(Op::Arith(ArithOp::NotEqual));

        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::Arith(ArithOp::NotEqual));

        let mut vm = Vm::from_program(prog);
        vm.exec(None);

        assert_eq!(vm.pop(), 0);
        assert_eq!(vm.pop(), 1);
        assert_eq!(vm.pop(), 1);
        assert_eq!(vm.pop(), 0);
    }
    #[test]
    fn arith_rel() {
        let mut prog = Program::new();

        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::PushImmediate(3));
        prog.code.push(Op::Arith(ArithOp::LessThan));

        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::Arith(ArithOp::LessThan));

        prog.code.push(Op::PushImmediate(3));
        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::Arith(ArithOp::LessThan));

        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::PushImmediate(3));
        prog.code.push(Op::Arith(ArithOp::LessEqual));

        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::Arith(ArithOp::LessEqual));

        prog.code.push(Op::PushImmediate(3));
        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::Arith(ArithOp::LessEqual));

        let mut vm = Vm::from_program(prog);
        vm.exec(None);

        assert_eq!(vm.pop(), 0);
        assert_eq!(vm.pop(), 1);
        assert_eq!(vm.pop(), 1);
        assert_eq!(vm.pop(), 0);
        assert_eq!(vm.pop(), 0);
        assert_eq!(vm.pop(), 1);
    }
    #[test]
    fn arith_bool() {
        let mut prog = Program::new();

        prog.code.push(Op::PushImmediate(0));
        prog.code.push(Op::PushImmediate(0));
        prog.code.push(Op::Arith(ArithOp::And));

        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::PushImmediate(0));
        prog.code.push(Op::Arith(ArithOp::And));

        prog.code.push(Op::PushImmediate(0));
        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::Arith(ArithOp::And));

        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::Arith(ArithOp::And));

        prog.code.push(Op::PushImmediate(0));
        prog.code.push(Op::PushImmediate(0));
        prog.code.push(Op::Arith(ArithOp::Or));

        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::PushImmediate(0));
        prog.code.push(Op::Arith(ArithOp::Or));

        prog.code.push(Op::PushImmediate(0));
        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::Arith(ArithOp::Or));

        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::Arith(ArithOp::Or));

        let mut vm = Vm::from_program(prog);
        vm.exec(None);

        assert_eq!(vm.pop(), 1);
        assert_eq!(vm.pop(), 1);
        assert_eq!(vm.pop(), 1);
        assert_eq!(vm.pop(), 0);

        assert_eq!(vm.pop(), 1);
        assert_eq!(vm.pop(), 0);
        assert_eq!(vm.pop(), 0);
        assert_eq!(vm.pop(), 0);
    }
    #[test]
    fn jump() {
        env_logger::init();
        info!("log");
        let mut prog = Program::new();
        prog.data.push(123);
        prog.data.push(666);
        prog.data.push(777);

        prog.code.push(Op::PushImmediate(1));
        prog.code.push(Op::PushImmediate(4));
        prog.code.push(Op::Jmp(Cond::NonZero));
        prog.code.push(Op::PushConst(2));
        prog.code.push(Op::PushImmediate(2));
        prog.code.push(Op::Jmp(Cond::Always));
        prog.code.push(Op::PushConst(1));
        prog.code.push(Op::Noop);
        println!("{}", serde_yaml::to_string(&prog).unwrap());

        let mut vm = Vm::from_program(prog);
        vm.exec(None);
        println!("{}", vm.pop());
    }
    #[test]
    fn int24() {
        let a: Uint24 = 10.into();
        let b: Uint24 = 0xAABBCC.into();
        let c: Uint24 = 0xFF.into();
        let d: Uint24 = 0xFF00.into();
        let e: Uint24 = 0xFF0000.into();
        let f: Uint24 = 0xFFFFFF.into();

        // print!("{:?}", b);
        let a1: u32 = a.into();
        let b1: u32 = b.into();
        let c1: u32 = c.into();
        let d1: u32 = d.into();
        let e1: u32 = e.into();
        let f1: u32 = f.into();
        assert_eq!(a1, 10);
        assert_eq!(b1, 0xAABBCC);
        assert_eq!(c1, 0xFF);
        assert_eq!(d1, 0xFF00);
        assert_eq!(e1, 0xFF0000);
        assert_eq!(f1, 0xFFFFFF);
    }
}
