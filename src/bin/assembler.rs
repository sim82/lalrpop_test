use lalrpop_test::{
    asm::{label_locations, xas, BytecodeEmit, Section},
    bytecode::{IoChannels, Op, Vm},
};
use std::{
    io::Read,
    sync::mpsc::{channel, Receiver, Sender},
};

fn main() {
    let mut code = String::new();
    std::io::stdin().lock().read_to_string(&mut code);
    let program = xas::ProgramParser::new().parse(&code[..]).unwrap();

    if let (Section::Data(data), Section::Code(stmts)) = (&program[0], &program[1]) {
        let labels = label_locations(stmts);
        let mut bc = Vec::new();
        for stmt in stmts {
            stmt.emit(&labels, &mut bc);
        }
        bc.push(Op::Noop);

        let mut vm = Vm::new();
        vm.data = data.clone();
        vm.code = bc;

        let (send, recv) = channel();
        let mut io_channels = IoChannels::new();
        io_channels.channels.push(send);
        vm.exec(Some(&io_channels));

        loop {
            if let Ok(v) = recv.try_recv() {
                println!("out: {}", v);
            } else {
                break;
            }
        }
    }
}
