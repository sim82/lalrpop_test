use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
};

use codegen::Context;
use cranelift::{
    codegen::{
        binemit::NullTrapSink,
        ir::{self, Value},
    },
    prelude::*,
};

use cranelift_module::{default_libcall_names, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};

use handy::{Handle, HandleMap};
use lalrpop_test::{
    ast::{Declaration, Expr, Opcode, Stmt, Toplevel},
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
    let mut emit_state = EmitStateGen::<_> {
        functions: Default::default(),
        scope_stack: Default::default(),
        module,
    };
    for d in program.decls.iter() {
        match d {
            Declaration::Function(name, args, body) => {
                let name = program.env.get(*name).unwrap();
                let mut sig = emit_state.module.make_signature();

                sig.returns.push(AbiParam::new(types::I64));
                for a in args {
                    sig.params.push(AbiParam::new(types::I64));
                }
                let func = emit_state
                    .module
                    .declare_function(name, Linkage::Export, &sig)
                    .unwrap();
                emit_state.functions.insert(name.clone(), func);

                program.emit_function(
                    &mut emit_state,
                    &mut ctx,
                    &mut func_ctx,
                    func,
                    sig,
                    args.clone(),
                    body,
                );
                let mut trap_sink = NullTrapSink {};
                emit_state
                    .module
                    .define_function(func, &mut ctx, &mut trap_sink)
                    .unwrap();
                emit_state.module.clear_context(&mut ctx);
            }
        }
    }

    if !program.toplevel.is_empty() {
        let mut sig = emit_state.module.make_signature();
        let func = emit_state
            .module
            .declare_function("main", Linkage::Export, &sig)
            .unwrap();
        sig.returns.push(AbiParam::new(types::I64));
        program.emit_function(
            &mut emit_state,
            &mut ctx,
            &mut func_ctx,
            func,
            sig,
            vec![],
            &Stmt::Block(program.toplevel.clone(), false),
        );
        let mut trap_sink = NullTrapSink {};
        emit_state
            .module
            .define_function(func, &mut ctx, &mut trap_sink)
            .unwrap();
        emit_state.module.clear_context(&mut ctx);
    }

    let product = emit_state.module.finish();
    let o = product.object.write().unwrap();
    let mut f = File::create("test.o").unwrap();
    f.write_all(&o).unwrap();
}

#[derive(Default)]
struct Program {
    env: HandleMap<String>,
    decls: Vec<Declaration>,
    toplevel: Vec<Stmt>,
}

trait EmitState {
    fn declare_func_in_func(&mut self, func_id: FuncId, bcx: &mut FunctionBuilder) -> ir::FuncRef;
    fn get_function(&self, name: &str) -> FuncId;
    fn get_scope_stack(&mut self) -> &ScopeStack;
    fn get_scope_stack_mut(&mut self) -> &mut ScopeStack;
}

#[derive(Default)]
struct ScopeStack {
    scopes: Vec<HashMap<String, Value>>,
}

impl ScopeStack {
    fn bind_value(&mut self, name: &str, v: Value) {
        self.scopes.last_mut().unwrap().insert(name.to_string(), v);
    }
    fn get_value(&self, name: &str) -> Value {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return *v;
            }
        }
        panic!("name '{}' not bound in {:?}", name, self.scopes)
    }

    fn push(&mut self) {
        self.scopes.push(Default::default())
    }
    fn pop(&mut self) {
        self.scopes.pop();
    }
}

struct EmitStateGen<M: Module> {
    functions: HashMap<String, FuncId>,
    scope_stack: ScopeStack,
    module: M,
}

impl<M: Module> EmitState for EmitStateGen<M> {
    fn declare_func_in_func(&mut self, func_id: FuncId, bcx: &mut FunctionBuilder) -> ir::FuncRef {
        self.module.declare_func_in_func(func_id, bcx.func)
    }

    fn get_function(&self, name: &str) -> FuncId {
        self.functions.get(name).unwrap().clone()
    }
    fn get_scope_stack(&mut self) -> &ScopeStack {
        &self.scope_stack
    }
    fn get_scope_stack_mut(&mut self) -> &mut ScopeStack {
        &mut self.scope_stack
    }
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
            ..Default::default()
        }
    }
    fn env_get(&self, h: &Handle) -> &str {
        self.env.get(*h).unwrap()
    }
    fn emit_expr(
        &self,
        bcx: &mut FunctionBuilder,
        emit_state: &mut dyn EmitState,
        expr: &Expr,
    ) -> Value {
        println!("emit_expr: {:?}", expr);
        match expr {
            Expr::Number(n) => bcx.ins().iconst(types::I64, *n),
            Expr::EnvLoad(h) => emit_state.get_scope_stack().get_value(self.env_get(h)),
            Expr::Op(e1, op, e2) => {
                let v1 = self.emit_expr(bcx, emit_state, &e1);
                let v2 = self.emit_expr(bcx, emit_state, &e2);
                match op {
                    Opcode::Add => bcx.ins().iadd(v1, v2),
                    Opcode::Mul => bcx.ins().imul(v1, v2),
                    _ => panic!("not implemented"),
                }
            }
            Expr::Call(name, args) => {
                let func_id = emit_state.get_function(self.env_get(name));

                let argsv = args
                    .iter()
                    .map(|arg| self.emit_expr(bcx, emit_state, arg))
                    .collect::<Vec<_>>();
                let local_func = emit_state.declare_func_in_func(func_id, bcx);
                let call = bcx.ins().call(local_func, &argsv[..]);
                let results = bcx.inst_results(call);
                assert_eq!(results.len(), 1);
                results[0].clone()
            }
            _ => panic!("not implemented"),
            // Expr::Error => {}
        }
    }

    fn emit_stmt(&self, bcx: &mut FunctionBuilder, emit_state: &mut dyn EmitState, stmt: &Stmt) {
        println!("emit_stmt: {:?}", stmt);
        match stmt {
            Stmt::LetBinding(name, expr) => {
                // let slot = bcx.create_stack_slot(StackSlotData {
                //     kind: StackSlotKind::ExplicitSlot,
                //     size: 8,
                //     offset: None,
                // });
                let v = self.emit_expr(bcx, emit_state, expr);
                // bcx.ins().stack_store(v, slot, 0);
                emit_state
                    .get_scope_stack_mut()
                    .bind_value(self.env_get(name), v);
            }
            Stmt::Assign(_, _, _) => {}
            Stmt::Print(_) => {}
            Stmt::IfElse(_, _, _) => {}
            Stmt::While(_, _) => {}
            Stmt::Block(stmts, _) => {
                emit_state.get_scope_stack_mut().push();
                for s in stmts.iter() {
                    self.emit_stmt(bcx, emit_state, s);
                }
                emit_state.get_scope_stack_mut().pop();
            }
            Stmt::Call(expr) => {
                self.emit_expr(bcx, emit_state, expr);
            }
            Stmt::Return(e) => {
                let v = self.emit_expr(bcx, emit_state, e);
                bcx.ins().return_(&[v]);
            }
        }
    }

    fn emit_function(
        &self,
        emit_state: &mut dyn EmitState,
        ctx: &mut Context,
        func_ctx: &mut FunctionBuilderContext,
        func: FuncId,
        sig: Signature,
        args: Vec<Handle>,
        body: &Stmt,
    ) {
        ctx.func.signature = sig;
        ctx.func.name = ExternalName::user(0, func.as_u32());
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, func_ctx);
        let block = bcx.create_block();
        emit_state.get_scope_stack_mut().push();
        bcx.switch_to_block(block);

        bcx.append_block_params_for_function_params(block);
        // bcx.append_block_params_for_function_params(block);

        for (i, arg) in args.iter().enumerate() {
            emit_state
                .get_scope_stack_mut()
                .bind_value(self.env_get(arg), bcx.block_params(block)[i])
        }

        self.emit_stmt(&mut bcx, emit_state, body);
        // bcx.ins().return_(&[arg]);
        bcx.seal_all_blocks();
        bcx.finalize();
        emit_state.get_scope_stack_mut().pop();
    }
}
