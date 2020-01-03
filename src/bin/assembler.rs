use lalrpop_test::{
    asm::{extract_constants, label_locations, xas, BytecodeEmit, Section},
    bytecode::{Op, Program},
};
use std::io::Read;

fn main() {
    env_logger::init();

    let mut code = String::new();
    std::io::stdin().lock().read_to_string(&mut code).unwrap();
    let program = xas::ProgramParser::new().parse(&code[..]).unwrap();

    if let (Section::Data(data), Section::Code(stmts)) = (&program[0], &program[1]) {
        let mut data = data.clone();
        let labels = label_locations(stmts);
        extract_constants(stmts, &mut data);
        let mut code = Vec::new();
        for stmt in stmts {
            stmt.emit(&labels, &data, &mut code);
        }
        code.push(Op::Noop);

        // let mut vm = Vm::new();
        // vm.data = data.clone();
        // vm.code = bc;
        // println!("data: {:?}", vm.data);
        // println!("code: {:?}", vm.code);
        let prog = Program { data, code };
        serde_yaml::to_writer(&mut std::io::stdout().lock(), &prog).unwrap();
    }
}
