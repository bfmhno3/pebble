struct Expr;

fn walk_expr<V: Visit + ?Sized>(visitor: &mut V, expr: &Expr) {
    visitor.visit_expr(expr);
}

trait Visit {
    fn visit_expr(&mut self, _expr: &Expr) {}
}

struct Counter(usize);
impl Visit for Counter {
    fn visit_expr(&mut self, _expr: &Expr) {
        self.0 += 1;
    }
}

fn main() {
    let mut counter = Counter(0);
    walk_expr(&mut counter, &Expr);
    println!("static dispatch: {}", counter.0);

    let dynamic: &mut dyn Visit = &mut counter;
    walk_expr(dynamic, &Expr);
    println!("dynamic dispatch: {}", counter.0);
}
