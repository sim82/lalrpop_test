use lalrpop_test::bytecode::{IoChannels, Program, Vm};
use std::sync::mpsc::channel;

fn main() {
    let prog: Program = serde_yaml::from_reader(&mut std::io::stdin().lock()).unwrap();
    let mut vm = Vm::from_program(prog);
    vm.max_ops = Some(1000);

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
