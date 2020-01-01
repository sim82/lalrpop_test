use handy::HandleMap;
use lalrpop_test::{
    asm::{self, Disass},
    ast::{Expr, Ident, Opcode, Stmt},
    lang1,
};
use log::debug;
use std::collections::HashMap;
use std::io::Read;

struct CodeGen<'env> {
    bindings: HashMap<Ident, usize>,
    stack_top: usize,
    asm_out: Vec<asm::Stmt>,
    label_count: HashMap<String, usize>,
    env: &'env HandleMap<&'env str>,
}

impl<'env> CodeGen<'env> {
    pub fn new(env: &'env HandleMap<&'env str>) -> CodeGen<'env> {
        CodeGen {
            bindings: HashMap::new(),
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
                self.bindings.insert(ident.clone(), self.stack_top);
                debug!(
                    "let binding: {} {}",
                    self.env.get(*ident).unwrap(),
                    self.stack_top
                );
                self.emit_expr(expr);
                self.stack_top += 1;
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
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.emit(s);
                }
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
                if let Some(pos) = self.bindings.get(ident) {
                    let offs = self.stack_top - pos - 1;
                    self.asm_out.push(asm::Stmt::PushStack(offs as i64));
                } else {
                    panic!("unknown binding: {}", self.env.get(*ident).unwrap());
                }
            }
            Expr::Op(a, op, b) => {
                self.emit_expr(a);
                self.emit_expr(b);

                let op = match op {
                    Opcode::Add => asm::ArithOp::Add,
                    Opcode::Sub => asm::ArithOp::Sub,
                    Opcode::Mul => asm::ArithOp::Mul,
                    Opcode::Div => asm::ArithOp::Div,
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
