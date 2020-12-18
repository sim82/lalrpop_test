use handy::HandleMap;
use lalrpop_test::{
    asm::{self, Disass},
    ast::{Declaration, Expr, Ident, Opcode, Stmt, Toplevel},
    lang1,
};
use log::debug;
use std::collections::HashMap;
use std::io::Read;

struct StackFrame {
    bindings: HashMap<Ident, usize>,
    stack_top: usize,
    bindings_top: usize,
}

impl StackFrame {
    fn new(stack_top: usize) -> StackFrame {
        StackFrame {
            bindings: HashMap::new(),
            stack_top,
            bindings_top: stack_top,
        }
    }
}

struct ScopeStack {
    frames: Vec<StackFrame>,
}

impl ScopeStack {
    fn new() -> ScopeStack {
        ScopeStack {
            frames: vec![StackFrame::new(0)],
        }
    }

    fn push_frame(&mut self) {
        let top = self.frames.last().unwrap().stack_top;
        self.frames.push(StackFrame::new(top));
    }
    fn pop_frame(&mut self) -> usize {
        let top = self.frames.last().unwrap().stack_top;
        self.frames.pop();
        let new_top = self.frames.last().unwrap().stack_top;
        assert!(top >= new_top);
        top - new_top
    }
    fn add_binding(&mut self, ident: Ident) {
        let frame = self.frames.last_mut().unwrap();
        assert!(frame.stack_top > 0);
        frame.bindings.insert(ident, frame.stack_top - 1);
        // frame.stack_top += 1;
        debug!(
            "add binding: {:?} {} -> {}",
            ident,
            frame.bindings_top,
            frame.stack_top - 1
        );

        frame.bindings_top = frame.stack_top - 1;
    }
    fn push_local(&mut self) {
        let frame = self.frames.last_mut().unwrap();

        debug!("push local {} -> {}", frame.stack_top, frame.stack_top + 1);
        frame.stack_top += 1;
    }
    fn pop_local(&mut self, num: usize) {
        let frame = self.frames.last_mut().unwrap();
        debug!(
            "pop local {} -> {}",
            frame.stack_top,
            frame.stack_top as i64 - num as i64
        );

        assert!(frame.stack_top - num >= frame.bindings_top);
        frame.stack_top -= num;
    }
    fn resolve(&self, ident: &Ident) -> Option<usize> {
        let top = self.frames.last().unwrap().stack_top;
        for frame in self.frames.iter().rev() {
            if let Some(pos) = frame.bindings.get(ident) {
                assert!(top > *pos);
                debug!(
                    "resolve local: {:?} {} {} -> {}",
                    ident,
                    top,
                    pos,
                    top - pos - 1
                );
                return Some(top - pos - 1);
            }
        }
        return None;
    }
}

struct CodeGen<'env> {
    scopes: ScopeStack,
    asm_out: Vec<asm::Stmt>,
    label_count: HashMap<String, usize>,
    env: &'env HandleMap<&'env str>,
}

impl<'env> CodeGen<'env> {
    pub fn new(env: &'env HandleMap<&'env str>) -> CodeGen<'env> {
        CodeGen {
            scopes: ScopeStack::new(),
            asm_out: Vec::new(),
            label_count: HashMap::new(),
            env,
        }
    }
    fn alloc_label(&mut self, template: &str) -> String {
        let count = self.label_count.entry(template.into()).or_insert(0);
        let c = *count;
        *count += 1;
        format!("{}{}", template, c)
    }
    fn emit(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::LetBinding(ident, expr) => {
                // self.bindings.insert(ident.clone(), self.stack_top);
                self.emit_expr(expr);
                self.scopes.add_binding(ident.clone());
            }
            Stmt::Assign(ident, expr, op) => {
                assert!(op.is_none());
                if let Some(offs) = self.scopes.resolve(ident) {
                    self.emit_expr(expr);
                    // self.asm_out.push(asm::Stmt::PushInline(offs as i64));
                    self.asm_out.push(asm::Stmt::Move(offs as i64));
                    self.scopes.pop_local(1);
                } else {
                    panic!("unknown binding: {}", self.env.get(*ident).unwrap());
                }
            }
            Stmt::IfElse(expr, if_stmt, None) => {
                self.emit_expr(expr);
                let label = self.alloc_label("if_end");
                self.asm_out
                    .push(asm::Stmt::Jmp(asm::Cond::Zero, Some(label.clone())));
                self.scopes.pop_local(1);
                self.emit(if_stmt);
                self.asm_out.push(asm::Stmt::Label(label));
            }
            Stmt::IfElse(expr, if_stmt, Some(else_stmt)) => {
                self.emit_expr(expr);
                let else_label = self.alloc_label("else");
                self.asm_out
                    .push(asm::Stmt::Jmp(asm::Cond::Zero, Some(else_label.clone())));
                self.scopes.pop_local(1);
                let dbg_label = self.alloc_label("if_else_begin");
                self.asm_out.push(asm::Stmt::Label(dbg_label));

                self.emit(if_stmt);
                let end_label = self.alloc_label("else_end");
                self.asm_out
                    .push(asm::Stmt::Jmp(asm::Cond::Always, Some(end_label.clone())));
                self.asm_out.push(asm::Stmt::Label(else_label));
                self.emit(else_stmt);
                self.asm_out.push(asm::Stmt::Label(end_label));
            }
            Stmt::While(expr, body) => {
                let start_label = self.alloc_label("while");
                self.asm_out.push(asm::Stmt::Label(start_label.clone()));
                self.emit_expr(expr);
                let exit_label = self.alloc_label("while_end");
                self.asm_out
                    .push(asm::Stmt::Jmp(asm::Cond::Zero, Some(exit_label.clone())));
                self.scopes.pop_local(1);

                self.emit(body);
                self.asm_out
                    .push(asm::Stmt::Jmp(asm::Cond::Always, Some(start_label)));
                self.asm_out.push(asm::Stmt::Label(exit_label));
            }
            Stmt::Block(stmts, cleanup_stack) => {
                self.scopes.push_frame();
                for s in stmts {
                    self.emit(s);
                }
                let mut num_pop = self.scopes.pop_frame();
                if !*cleanup_stack {
                    assert!(num_pop >= 1); // we must leave on element on the stack as return value
                    num_pop -= 1;
                }
                debug!("scope exit: {} {:?}", num_pop, cleanup_stack);

                self.asm_out.push(asm::Stmt::Pop(num_pop as i64));
            }
            Stmt::Print(exprs) => {
                for e in exprs {
                    self.emit_expr(e);
                    self.asm_out.push(asm::Stmt::Output(0));
                    self.scopes.pop_local(1);
                }
            }
            Stmt::Call(expr) => {
                self.emit_expr(expr);
                self.asm_out.push(asm::Stmt::Pop(1));
                self.scopes.pop_local(1);
            }
            Stmt::Return(e) => {
                self.emit_expr(e);
            }
        }
    }
    fn emit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Number(v) => {
                self.asm_out.push(asm::Stmt::PushInline(*v));
                self.scopes.push_local();
            }
            Expr::EnvLoad(ident) => {
                if let Some(offs) = self.scopes.resolve(ident) {
                    self.asm_out.push(asm::Stmt::PushStack(offs as i64));
                    self.scopes.push_local();
                } else {
                    panic!("unknown binding: {}", self.env.get(*ident).unwrap());
                }
            }
            Expr::Op(a, op, b) => {
                match op {
                    Opcode::GreaterThan | Opcode::GreaterEqual => {
                        self.emit_expr(b);
                        self.emit_expr(a);
                    }
                    _ => {
                        self.emit_expr(a);
                        self.emit_expr(b);
                    }
                }
                let op = match op {
                    Opcode::Add => asm::ArithOp::Add,
                    Opcode::Sub => asm::ArithOp::Sub,
                    Opcode::Mul => asm::ArithOp::Mul,
                    Opcode::Div => asm::ArithOp::Div,
                    Opcode::Or => asm::ArithOp::Or,
                    Opcode::And => asm::ArithOp::And,
                    Opcode::Equal => asm::ArithOp::Equal,
                    Opcode::NotEqual => asm::ArithOp::NotEqual,
                    Opcode::LessThan | Opcode::GreaterThan => asm::ArithOp::LessThan,
                    Opcode::LessEqual | Opcode::GreaterEqual => asm::ArithOp::LessEqual,
                };
                self.scopes.pop_local(2);
                self.asm_out.push(asm::Stmt::Arith(op));
                self.scopes.push_local();
            }
            Expr::Call(name, exprs) => {
                self.asm_out.push(asm::Stmt::PushInline(0));
                self.scopes.push_local();
                for e in exprs {
                    self.emit_expr(e);
                }
                let name: String = format!("func_{}", *self.env.get(*name).unwrap());
                self.asm_out.push(asm::Stmt::Call(name));
                self.asm_out.push(asm::Stmt::Pop(exprs.len() as i64));
                self.scopes.pop_local(exprs.len());
                // self.scopes.push_local();
            }
            Expr::Error => panic!("found Expr::Error in emit_expr"),
        }
    }
    fn emit_return(&mut self) {
        self.asm_out.push(asm::Stmt::Jmp(asm::Cond::Always, None));
    }
}

fn main() {
    env_logger::init();

    let mut code = String::new();
    std::io::stdin().lock().read_to_string(&mut code).unwrap();

    let mut env = HandleMap::new();
    let mut errors = Vec::new();
    let program = lang1::ProgramParser::new()
        .parse(&mut env, &mut errors, &code[..])
        .unwrap();

    let mut stmts = Vec::new();
    let mut decls = Vec::new();
    for p in program {
        match p {
            Toplevel::Stmt(s) => stmts.push(s),
            Toplevel::Declaration(d) => decls.push(d),
        }
    }

    let mut codegen = CodeGen::new(&env);

    if !decls.is_empty() {
        codegen
            .asm_out
            .push(asm::Stmt::Jmp(asm::Cond::Always, Some("entry".into())));
    }

    for d in &decls {
        match d {
            Declaration::Function(name, args, body) => {
                codegen.asm_out.push(asm::Stmt::Label(format!(
                    "func_{}",
                    env.get(*name).unwrap()
                )));
                codegen.scopes.push_frame();
                for a in args {
                    codegen.scopes.push_local();
                    codegen.scopes.add_binding(a.clone());
                }
                codegen.scopes.push_local(); // for return address
                codegen.emit(body);
                codegen.asm_out.push(asm::Stmt::Move(args.len() as i64 + 1));
                codegen.emit_return();
                codegen.scopes.pop_frame();
            }
        }
    }
    codegen.asm_out.push(asm::Stmt::Label("entry".into()));
    for s in &stmts {
        codegen.emit(s);
    }
    // for d in &decls {
    //     println!("decl: {:?}", d);
    // }
    asm::Section::Data(Vec::new()).print_lines(&mut std::io::stdout().lock());
    asm::Section::Code(codegen.asm_out).print_lines(&mut std::io::stdout().lock());
}

#[cfg(test)]
mod compiler_test {
    use super::asm::{ArithOp, Cond, Stmt};
    use super::lang1;
    use super::{CodeGen, Toplevel};
    use handy::HandleMap;
    #[test]
    fn test_basic() {
        let code = include_str!("test_compiler_basic.l1");
        let mut env = HandleMap::new();
        let mut errors = Vec::new();
        let program = lang1::ProgramParser::new()
            .parse(&mut env, &mut errors, code)
            .unwrap();

        let mut codegen = CodeGen::new(&env);

        for p in &program {
            match p {
                Toplevel::Stmt(s) => codegen.emit(s),
                _ => (),
                // Toplevel::Declaration(d) => decls.push(d),
            }
        }
        println!("asm: {:?}", codegen.asm_out);
        let asm_ref = [
            // Stmt::Jmp(Cond::Always, Some("entry".into())),
            // Stmt::Label("entry".into()),
            Stmt::PushInline(123),
            Stmt::PushInline(321),
            Stmt::PushInline(432),
            Stmt::PushStack(1),
            Stmt::Output(0),
            Stmt::Pop(2),
            Stmt::PushStack(0),
            Stmt::Output(0),
            Stmt::Pop(0),
            Stmt::PushStack(0),
            Stmt::Output(0),
            Stmt::Label("while0".into()),
            Stmt::PushStack(0),
            Stmt::PushInline(0),
            Stmt::Arith(ArithOp::NotEqual),
            Stmt::Jmp(Cond::Zero, Some("while_end0".into())),
            Stmt::PushInline(1),
            Stmt::PushStack(1),
            Stmt::PushInline(1),
            Stmt::Arith(ArithOp::Sub),
            Stmt::Move(1),
            Stmt::Pop(1),
            Stmt::Jmp(Cond::Always, Some("while0".into())),
            Stmt::Label("while_end0".into()),
        ];
        assert_eq!(codegen.asm_out[..], asm_ref);
    }
}
