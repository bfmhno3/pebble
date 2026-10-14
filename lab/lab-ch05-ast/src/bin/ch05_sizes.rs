use pebble_ast::{Expr, Ident, Stmt};
use std::mem::size_of;

fn main() {
    println!("Span         = {} bytes", size_of::<pebble_span::Span>());
    println!("Ident        = {} bytes", size_of::<Ident>());
    println!("Stmt         = {} bytes", size_of::<Stmt>());
    println!("Expr         = {} bytes", size_of::<Expr>());
    println!("Box<Expr>    = {} bytes", size_of::<Box<Expr>>());
    println!("Vec<Expr>    = {} bytes", size_of::<Vec<Expr>>());
    println!("(Expr, Expr) = {} bytes", size_of::<(Expr, Expr)>());
}
