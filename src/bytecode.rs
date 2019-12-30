use serde::{Deserialize, Serialize};
// use
#[derive(Clone, Serialize, Deserialize, Debug)]
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

#[derive(Clone, Serialize, Deserialize, Debug)]
enum Op {
    PushConst(u16),
    PushStack(i16),
    // Arith(ArithOp),
    Add,
    Sub,
    Mul,
    Div,
}

struct Vm {
    data: Vec<i64>,
    stack: Vec<i64>,
    code: Vec<Op>,
    ip: usize,
}

impl Vm {
    fn new() -> Self {
        Vm {
            data: Vec::new(),
            stack: Vec::new(),
            code: Vec::new(),
            ip: 0,
        }
    }
    fn push(&mut self, v: i64) {
        self.stack.push(v);
    }
    fn pop(&mut self) -> i64 {
        self.stack.pop().unwrap()
    }
    fn peek(&self) -> i64 {
        *self.stack.last().unwrap()
    }
    pub fn exec(&mut self) {
        while self.ip < self.code.len() {
            let op = self.code[self.ip].clone();
            match op {
                Op::PushConst(offs) => {
                    self.push(self.data[offs as usize]);
                }
                Op::PushStack(offs) => {}
                // Op::Arith(op) => {
                //     let a = self.pop();
                //     let b = self.pop();
                //     self.push(op.eval(a, b));
                // }
                Op::Add | Op::Sub | Op::Mul | Op::Div => {
                    let a = self.pop();
                    let b = self.pop();
                    let c = match op {
                        Op::Add => a + b,
                        Op::Sub => a - b,
                        Op::Mul => a * b,
                        Op::Div => a / b,
                        _ => panic!("unreachable"),
                    };
                    self.push(c);
                }
            }
            self.ip += 1;
        }
    }
}

#[test]
fn basic() {
    let mut vm = Vm::new();
    vm.data.push(123);
    vm.data.push(666);
    vm.data.push(777);

    vm.code.push(Op::PushConst(0));
    vm.code.push(Op::PushConst(1));
    vm.code.push(Op::Add);

    vm.code.push(Op::PushConst(1));
    vm.code.push(Op::PushConst(2));

    vm.code.push(Op::Sub);

    vm.exec();
    println!("sub: {}", vm.pop());
    println!("add: {}", vm.pop());
    println!("{}", serde_yaml::to_string(&vm.code).unwrap());
    let bin = bincode::serialize(&vm.code).unwrap();
    println!("{} {:?}", bin.len(), bin);
    println!("{}", std::mem::size_of::<Op>());
}
