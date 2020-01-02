use handy::HandleMap;
use lalrpop_test::{
    asm::{self, Disass},
    ast::{Expr, Ident, Opcode, Stmt},
    lang1,
};
use log::debug;
use std::collections::HashMap;
use std::io::Read;

struct StackFrame {
    bindings: HashMap<Ident, usize>,
    stack_top: usize,
}

impl StackFrame {
    fn new(stack_top: usize) -> StackFrame {
        StackFrame {
            bindings: HashMap::new(),
            stack_top,
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

    fn push(&mut self) {
        let top = self.frames.last().unwrap().stack_top;
        self.frames.push(StackFrame::new(top));
    }
    fn pop(&mut self) -> usize {
        let top = self.frames.last().unwrap().stack_top;
        self.frames.pop();
        let new_top = self.frames.last().unwrap().stack_top;
        assert!(top >= new_top);
        top - new_top
    }
    fn add_binding(&mut self, ident: Ident) {
        let frame = self.frames.last_mut().unwrap();
        frame.bindings.insert(ident, frame.stack_top);
        frame.stack_top += 1;
    }
    fn resolve(&self, ident: &Ident) -> Option<usize> {
        let top = self.frames.last().unwrap().stack_top;
        for frame in self.frames.iter().rev() {
            if let Some(pos) = frame.bindings.get(ident) {
                assert!(top > *pos);
                return Some(top - pos - 1);
            }
        }
        return None;
    }
}

struct CodeGen<'env> {
    scopes: ScopeStack,
    stack_top: usize,
    asm_out: Vec<asm::Stmt>,
    label_count: HashMap<String, usize>,
    env: &'env HandleMap<&'env str>,
}

impl<'env> CodeGen<'env> {
    pub fn new(env: &'env HandleMap<&'env str>) -> CodeGen<'env> {
        CodeGen {
            scopes: ScopeStack::new(),
            stack_top: 0,
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
                self.scopes.add_binding(ident.clone());
                // self.bindings.insert(ident.clone(), self.stack_top);
                debug!(
                    "let binding: {} {}",
                    self.env.get(*ident).unwrap(),
                    self.stack_top
                );
                self.emit_expr(expr);
                self.stack_top += 1;
            }
            Stmt::Assign(ident, expr, op) => {
                assert!(op.is_none());
                if let Some(offs) = self.scopes.resolve(ident) {
                    self.emit_expr(expr);
                    // self.asm_out.push(asm::Stmt::PushInline(offs as i64));
                    self.asm_out.push(asm::Stmt::Move(offs as i64));
                } else {
                    panic!("unknown binding: {}", self.env.get(*ident).unwrap());
                }
            }
            Stmt::IfElse(expr, if_stmt, None) => {
                self.emit_expr(expr);
                let label = self.alloc_label("if_end");
                self.asm_out
                    .push(asm::Stmt::Jmp(asm::Cond::Zero, label.clone()));
                self.emit(if_stmt);
                self.asm_out.push(asm::Stmt::Label(label));
            }
            Stmt::IfElse(expr, if_stmt, Some(else_stmt)) => {
                self.emit_expr(expr);
                let else_label = self.alloc_label("else");
                self.asm_out
                    .push(asm::Stmt::Jmp(asm::Cond::Zero, else_label.clone()));
                self.emit(if_stmt);
                let end_label = self.alloc_label("else_end");
                self.asm_out
                    .push(asm::Stmt::Jmp(asm::Cond::Always, end_label.clone()));
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
                    .push(asm::Stmt::Jmp(asm::Cond::Zero, exit_label.clone()));
                self.emit(body);
                self.asm_out
                    .push(asm::Stmt::Jmp(asm::Cond::Always, start_label));
                self.asm_out.push(asm::Stmt::Label(exit_label));
            }
            Stmt::Block(stmts) => {
                self.scopes.push();
                for s in stmts {
                    self.emit(s);
                }
                let num_pop = self.scopes.pop();
                self.asm_out.push(asm::Stmt::Pop(num_pop as i64));
                debug!("scope exit: {}", num_pop);
            }
            Stmt::Print(exprs) => {
                for e in exprs {
                    self.emit_expr(e);
                    self.asm_out.push(asm::Stmt::Output(0));
                }
            }
        }
    }
    fn emit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Number(v) => self.asm_out.push(asm::Stmt::PushInline(*v)),
            Expr::EnvLoad(ident) => {
                if let Some(offs) = self.scopes.resolve(ident) {
                    self.asm_out.push(asm::Stmt::PushStack(offs as i64));
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
                self.asm_out.push(asm::Stmt::Arith(op));
            }
            Expr::Error => panic!("found Expr::Error in emit_expr"),
        }
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

    let mut codegen = CodeGen::new(&env);
    for s in &program {
        codegen.emit(s);
    }
    asm::Section::Data(Vec::new()).print_lines(&mut std::io::stdout().lock());
    asm::Section::Code(codegen.asm_out).print_lines(&mut std::io::stdout().lock());
}
