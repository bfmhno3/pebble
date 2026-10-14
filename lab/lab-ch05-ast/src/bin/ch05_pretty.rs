use pebble_ast::{BinaryOp, Expr, Ident, Program, Span, Stmt, pretty};

fn var(name: &str) -> Expr {
    Expr::Var {
        name: Ident::new(name, Span::DUMMY),
        span: Span::DUMMY,
    }
}

fn main() {
    // fn add(a, b) { return a + b; }
    let program = Program::new(vec![Stmt::Fn {
        name: Ident::new("add", Span::DUMMY),
        params: vec![Ident::new("a", Span::DUMMY), Ident::new("b", Span::DUMMY)],
        body: vec![Stmt::Return {
            value: Some(Expr::Binary {
                op: BinaryOp::Add,
                lhs: Box::new(var("a")),
                rhs: Box::new(var("b")),
                span: Span::DUMMY,
            }),
            span: Span::DUMMY,
        }],
        span: Span::DUMMY,
    }]);
    print!("{}", pretty(&program));
}
