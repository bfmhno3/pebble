use std::mem::size_of;

enum Expr {
    Int(i64),
    Add(Box<Expr>, Box<Expr>),
    List(Vec<Expr>),
}

fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Int(n) => *n,
        Expr::Add(a, b) => eval(a) + eval(b),
        Expr::List(items) => items.iter().map(eval).sum(),
    }
}

fn main() {
    let tree = Expr::Add(
        Box::new(Expr::Int(1)),
        Box::new(Expr::Add(Box::new(Expr::Int(2)), Box::new(Expr::Int(3)))),
    );
    println!("eval = {}", eval(&tree));
    println!("Box<Expr>      = {} bytes", size_of::<Box<Expr>>());
    println!("Vec<Expr>      = {} bytes", size_of::<Vec<Expr>>());
    println!("Expr           = {} bytes", size_of::<Expr>());
    println!("Option<Box<Expr>> = {} bytes", size_of::<Option<Box<Expr>>>());
    println!("Option<i64>    = {} bytes", size_of::<Option<i64>>());
    let list = Expr::List(vec![Expr::Int(10), Expr::Int(20)]);
    println!("list = {}", eval(&list));
}
