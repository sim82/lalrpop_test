use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
struct Uint24([u8; 3]);

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
enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl ArithOp {
    fn eval(&self, a: i64, b: i64) -> i64 {
        match *self {
            ArithOp::Add => a + b,
            ArithOp::Sub => a - b,
            ArithOp::Mul => a * b,
            ArithOp::Div => a / b,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
enum JmpType {
    Abs,
    Rel,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
enum Cond {
    True,
    Zero,
    NonZero,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
enum Op {
    PushConst(u16),
    PushStack(i16),
    PushImmediate(i16),
    PushImmediate24(Uint24),
    Arith(ArithOp),
    Jmp(JmpType, Cond),
    Output(u16),
}

#[derive(Serialize, Deserialize)]
struct Vm {
    data: Vec<i64>,
    stack: Vec<i64>,
    code: Vec<Op>,
    ip: usize,
}
struct IoChannels {
    channels: Vec<Sender<i64>>,
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
    pub fn exec(&mut self, io: Option<&IoChannels>) {
        while self.ip < self.code.len() {
            let op = self.code[self.ip].clone();
            debug!("exec: {} {:?}", self.ip, op);
            match op {
                Op::PushConst(offs) => {
                    self.push(self.data[offs as usize]);
                }
                Op::PushStack(offs) => {}
                Op::Arith(op) => {
                    let a = self.pop();
                    let b = self.pop();
                    self.push(op.eval(a, b));
                }
                Op::PushImmediate(v) => self.push(v as i64),
                Op::PushImmediate24(v) => {
                    let v: u32 = v.into();
                    self.push(v as i64)
                }
                Op::Jmp(jmp_type, jmp_cond) => {
                    let cond = match jmp_cond {
                        Cond::True => true,
                        Cond::Zero => self.pop() == 0,
                        Cond::NonZero => self.pop() != 0,
                    };
                    let dst = self.pop();
                    debug!("jmp: {} {:?} {}", cond, jmp_type, dst);

                    if cond {
                        self.ip = match jmp_type {
                            JmpType::Abs => dst as usize,
                            JmpType::Rel => (self.ip as i64 + dst) as usize,
                        };
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
                        io.channels[channel as usize].send(v).unwrap();
                    }
                }
            }
            self.ip += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{channel, Receiver, Sender};
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

        vm.stack.push(4);
        vm.stack.push(0);
        vm.code.push(Op::Jmp(JmpType::Rel, Cond::NonZero));
        vm.code.push(Op::PushConst(2));
        vm.code.push(Op::PushImmediate(2));
        vm.code.push(Op::Jmp(JmpType::Rel, Cond::True));
        vm.code.push(Op::PushConst(1));
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
