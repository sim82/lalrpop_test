use std::{
    fs::File,
    io::{Read, Write},
    mem,
};

use codegen::Context;
use cranelift::{
    codegen::{
        binemit::NullTrapSink,
        ir::{Function, Value},
        verifier::verify_function,
    },
    prelude::isa::CallConv,
    prelude::types::*,
    prelude::*,
};

use cranelift_module::{default_libcall_names, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use cranelift_simplejit::{SimpleJITBuilder, SimpleJITModule, SimpleJITProduct};
use handy::{Handle, HandleMap};
use lalrpop_test::{
    asm::{self, Disass},
    ast::{Declaration, Expr, Ident, Opcode, Stmt, Toplevel},
    lang1,
};
fn main() {
    env_logger::init();
    test();

    if false {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        // FIXME set back to true once the x64 backend supports it.
        flag_builder.set("is_pic", "true").unwrap();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder.finish(settings::Flags::new(flag_builder));
        // let mut module = SimpleJITModule::new(SimpleJITBuilder::with_isa(isa, default_libcall_names()));
        let mut module =
            ObjectModule::new(ObjectBuilder::new(isa, "test.o", default_libcall_names()).unwrap());

        let mut ctx = module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        let mut sig_a = module.make_signature();
        sig_a.params.push(AbiParam::new(types::I32));
        sig_a.returns.push(AbiParam::new(types::I32));

        let mut sig_b = module.make_signature();
        sig_b.returns.push(AbiParam::new(types::I32));

        let func_a = module
            .declare_function("a", Linkage::Export, &sig_a)
            .unwrap();
        let func_b = module
            .declare_function("b", Linkage::Export, &sig_b)
            .unwrap();

        ctx.func.signature = sig_a;
        ctx.func.name = ExternalName::user(0, func_a.as_u32());
        {
            let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
            let block = bcx.create_block();

            bcx.switch_to_block(block);
            bcx.append_block_params_for_function_params(block);
            let param = bcx.block_params(block)[0];
            let cst = bcx.ins().iconst(types::I32, 37);
            let add = bcx.ins().iadd(cst, param);
            bcx.ins().return_(&[add]);
            bcx.seal_all_blocks();
            bcx.finalize();
        }
        println!("{}", ctx.func.display(None));
        let mut trap_sink = NullTrapSink {};
        module
            .define_function(func_a, &mut ctx, &mut trap_sink)
            .unwrap();
        module.clear_context(&mut ctx);

        ctx.func.signature = sig_b;
        ctx.func.name = ExternalName::user(0, func_b.as_u32());
        {
            let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
            let block = bcx.create_block();

            bcx.switch_to_block(block);
            let local_func = module.declare_func_in_func(func_a, &mut bcx.func);
            let arg = bcx.ins().iconst(types::I32, 5);
            let call = bcx.ins().call(local_func, &[arg]);
            let value = {
                let results = bcx.inst_results(call);
                assert_eq!(results.len(), 1);
                results[0].clone()
            };
            bcx.ins().return_(&[value]);
            bcx.seal_all_blocks();
            bcx.finalize();
        }
        println!("{}", ctx.func.display(None));

        module
            .define_function(func_b, &mut ctx, &mut trap_sink)
            .unwrap();
        module.clear_context(&mut ctx);

        // // Perform linking.
        // module.finalize_definitions();

        // // Get a raw pointer to the generated code.
        // let code_b = module.get_finalized_function(func_b);

        // // Cast it to a rust function pointer type.
        // let ptr_b = unsafe { mem::transmute::<_, fn() -> u32>(code_b) };

        // // Call it!
        // let res = ptr_b();
        // assert_eq!(res, 42);

        let product = module.finish();
        let o = product.object.write().unwrap();
        let mut f = File::create("test.o").unwrap();
        f.write_all(&o).unwrap();
    }
}

fn test() {
    let program = Program::parse_stdin();
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    // FIXME set back to true once the x64 backend supports it.
    flag_builder.set("is_pic", "true").unwrap();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        panic!("host machine is not supported: {}", msg);
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    // let mut module = SimpleJITModule::new(SimpleJITBuilder::with_isa(isa, default_libcall_names()));
    let mut module =
        ObjectModule::new(ObjectBuilder::new(isa, "test.o", default_libcall_names()).unwrap());

    let mut ctx = module.make_context();
    let mut func_ctx = FunctionBuilderContext::new();
    for d in program.decls.iter() {
        match d {
            Declaration::Function(name, args, body) => {
                program.emit_function(
                    &mut module,
                    &mut ctx,
                    &mut func_ctx,
                    *name,
                    args.clone(),
                    body,
                );
            }
        }
    }
    let product = module.finish();
    let o = product.object.write().unwrap();
    let mut f = File::create("test.o").unwrap();
    f.write_all(&o).unwrap();
}

struct Program {
    env: HandleMap<String>,
    decls: Vec<Declaration>,
    toplevel: Vec<Stmt>,
}

impl Program {
    fn parse_stdin() -> Self {
        let mut code = String::new();
        std::io::stdin().lock().read_to_string(&mut code).unwrap();

        let mut env = HandleMap::new();
        let mut errors = Vec::new();
        let program = lang1::ProgramParser::new()
            .parse(&mut env, &mut errors, &code[..])
            .unwrap();

        let mut decls = Vec::new();
        let mut toplevel = Vec::new();
        for p in program {
            match p {
                Toplevel::Stmt(s) => toplevel.push(s),
                Toplevel::Declaration(d) => decls.push(d),
            }
        }
        Program {
            env,
            decls,
            toplevel,
        }
    }
    fn emit_expr(&self, bcx: &mut FunctionBuilder, expr: &Expr) -> Value {
        match expr {
            Expr::Number(n) => bcx.ins().iconst(types::I64, *n),
            Expr::EnvLoad(_) => bcx.ins().iconst(types::I64, 0),
            Expr::Op(e1, op, e2) => {
                let v1 = self.emit_expr(bcx, &e1);
                let v2 = self.emit_expr(bcx, &e2);
                match op {
                    Opcode::Add => bcx.ins().iadd(v1, v2),
                    Opcode::Mul => bcx.ins().imul(v1, v2),
                    _ => panic!("not implemented"),
                }
            }
            Expr::Call(_, _) => bcx.ins().iconst(types::I64, 0),
            _ => panic!("not implemented"),
            // Expr::Error => {}
        }
    }

    fn emit_stmt(&self, bcx: &mut FunctionBuilder, stmt: &Stmt) {
        match stmt {
            Stmt::LetBinding(_, _) => {}
            Stmt::Assign(_, _, _) => {}
            Stmt::Print(_) => {}
            Stmt::IfElse(_, _, _) => {}
            Stmt::While(_, _) => {}
            Stmt::Block(stmts, b) => {
                for s in stmts.iter() {
                    self.emit_stmt(bcx, s);
                }
            }
            Stmt::Call(_) => {}
            Stmt::Return(e) => {
                let v = self.emit_expr(bcx, e);
                bcx.ins().return_(&[v]);
            }
        }
    }

    fn emit_function<M: Module>(
        &self,
        module: &mut M,
        ctx: &mut Context,
        func_ctx: &mut FunctionBuilderContext,
        name: Handle,
        args: Vec<Handle>,
        body: &Stmt,
    ) {
        let mut sig = module.make_signature();

        let func = module
            .declare_function(self.env.get(name).unwrap(), Linkage::Export, &sig)
            .unwrap();

        sig.returns.push(AbiParam::new(types::I64));
        for a in args {
            sig.params.push(AbiParam::new(types::I32));
        }

        ctx.func.signature = sig;
        ctx.func.name = ExternalName::user(0, func.as_u32());
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);

        // let arg = bcx.ins().iconst(types::I32, 5);

        bcx.append_block_params_for_function_params(block);
        self.emit_stmt(&mut bcx, body);
        // bcx.ins().return_(&[arg]);
        bcx.seal_all_blocks();
        bcx.finalize();
        let mut trap_sink = NullTrapSink {};
        module.define_function(func, ctx, &mut trap_sink).unwrap();
        module.clear_context(ctx);
    }
}
