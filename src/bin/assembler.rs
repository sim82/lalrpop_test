use lalrpop_test::{
    asm::{extract_constants, label_locations, xas, BytecodeEmit, Section},
    bytecode::{IoChannels, Op, Vm},
};
use std::{io::Read, sync::mpsc::channel};

fn main() {
    env_logger::init();

    let mut code = String::new();
    std::io::stdin().lock().read_to_string(&mut code).unwrap();
    let program = xas::ProgramParser::new().parse(&code[..]).unwrap();

    if let (Section::Data(data), Section::Code(stmts)) = (&program[0], &program[1]) {
        let mut data = data.clone();
        let labels = label_locations(stmts);
        extract_constants(stmts, &mut data);
        let mut bc = Vec::new();
        for stmt in stmts {
            stmt.emit(&labels, &data, &mut bc);
        }
        bc.push(Op::Noop);

        let mut vm = Vm::new();
        vm.data = data.clone();
        vm.code = bc;
        println!("data: {:?}", vm.data);
        println!("code: {:?}", vm.code);

        let (send, recv) = channel();
        let mut io_channels = IoChannels::new();
        io_channels.channels.push(send);
        vm.exec(Some(&io_channels));
        let mut num_out = 0;
        loop {
            if let Ok(v) = recv.try_recv() {
                println!("out: {}", v);
                num_out += 1;
            } else {
                break;
            }
        }
        println!("num output: {}", num_out);
        println!("num ops: {}", vm.num_ops);
    }
}
