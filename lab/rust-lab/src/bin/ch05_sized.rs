struct Expr;

fn walk_expr<V: Visit>(visitor: &mut V, expr: &Expr) {
    let _ = (visitor, expr);
}

trait Visit {
    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr(self, expr);
    }
}

fn main() {}
