use pebble_ast::{Expr, Ident, Program, Span, Stmt, Visit, walk_expr, walk_stmt};

#[derive(Default)]
struct VarCounter {
    reads: usize,
    lets: usize,
}

impl Visit for VarCounter {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        if matches!(stmt, Stmt::Let { .. }) {
            self.lets += 1;
        }
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        if matches!(expr, Expr::Var { .. }) {
            self.reads += 1;
        }
        walk_expr(self, expr);
    }
}

fn var(name: &str) -> Expr {
    Expr::Var { name: Ident::new(name, Span::DUMMY), span: Span::DUMMY }
}

fn main() {
    // let y = x + x;
    let program = Program::new(vec![Stmt::Let {
        name: Ident::new("y", Span::DUMMY),
        value: Expr::Binary {
            op: pebble_ast::BinaryOp::Add,
            lhs: Box::new(var("x")),
            rhs: Box::new(var("x")),
            span: Span::DUMMY,
        },
        span: Span::DUMMY,
    }]);
    let mut counter = VarCounter::default();
    counter.visit_program(&program);
    println!("lets = {}, reads = {}", counter.lets, counter.reads);

    let dynamic: &mut dyn Visit = &mut counter;
    dynamic.visit_program(&program);
    println!("through dyn Visit: lets = {}, reads = {}", counter.lets, counter.reads);
}
