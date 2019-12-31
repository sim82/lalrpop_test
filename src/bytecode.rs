use log::{debug, info};
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
}

impl ArithOp {
    pub fn eval(&self, a: i64, b: i64) -> i64 {
        match *self {
            ArithOp::Add => a + b,
            ArithOp::Sub => a - b,
            ArithOp::Mul => a * b,
            ArithOp::Div => a / b,
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
    Arith(ArithOp),
    Jmp(Cond),
    Output(u16),
    Pop(PopMode),
    Break,
}

#[derive(Serialize, Deserialize)]
pub struct Vm {
    pub data: Vec<i64>,
    stack: Vec<i64>,
    pub code: Vec<Op>,
    ip: usize,
    pub num_ops: usize,
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
        if offs < 0 || offs as usize >= self.stack.len() {
            panic!("stack underflow: {} (of {})", offs, self.stack.len());
        }
        self.stack[self.stack.len() - 1 - offs as usize]
    }
    pub fn exec(&mut self, io: Option<&IoChannels>) {
        while self.ip < self.code.len() {
            let op = self.code[self.ip].clone();
            self.num_ops += 1;
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
                        //  match jmp_type {
                        //     JmpType::Abs => dst as usize,
                        //     JmpType::Rel => (self.ip as i64 + dst) as usize,
                        // };
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
    use super::*;
    use std::sync::mpsc::channel;
    #[test]
    fn arith() {
        let mut vm = Vm::new();
        vm.data.push(123);
        vm.data.push(666);
        vm.data.push(777);

        vm.code.push(Op::PushConst(0));
        vm.code.push(Op::PushConst(1));
        // vm.code.push(Op::Add);
        vm.code.push(Op::Arith(ArithOp::Add));

        vm.code.push(Op::PushConst(1));
        // vm.code.push(Op::PushConst(2));
        // vm.code.push(Op::PushImmediate24(0xaabbcc.into()));
        vm.code.push(Op::PushImmediate(1666));

        // vm.code.push(Op::Sub);
        vm.code.push(Op::Arith(ArithOp::Sub));
        vm.code.push(Op::Output(0));
        vm.code.push(Op::Output(0));
        let (sender, receiver) = channel();
        let mut io = IoChannels::new();
        io.channels.push(sender);
        vm.exec(Some(&io));
        println!(
            "sub: {} add: {}",
            receiver.recv().unwrap(),
            receiver.recv().unwrap()
        );
        // println!("sub: {}", vm.pop());
        // println!("add: {}", vm.pop());

        println!("{}", serde_yaml::to_string(&vm).unwrap());
        let bin = bincode::serialize(&vm.code).unwrap();
        println!("{} {:?}", bin.len(), bin);
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
    fn jump() {
        env_logger::init();
        info!("log");
        let mut vm = Vm::new();
        vm.data.push(123);
        vm.data.push(666);
        vm.data.push(777);

        vm.code.push(Op::PushImmediate(1));
        vm.code.push(Op::PushImmediate(4));
        vm.code.push(Op::Jmp(Cond::NonZero));
        vm.code.push(Op::PushConst(2));
        vm.code.push(Op::PushImmediate(2));
        vm.code.push(Op::Jmp(Cond::Always));
        vm.code.push(Op::PushConst(1));
        vm.code.push(Op::Noop);
        println!("{}", serde_yaml::to_string(&vm).unwrap());

        vm.exec(None);
        println!("{}", vm.pop());
    }
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
