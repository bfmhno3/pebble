use lab_macros_debug::Spanned;
use pebble_span::Span;

#[derive(Spanned)]
struct Ident {
    name: String,
    span: Span,
}

#[derive(Spanned)]
enum Expr {
    Int { value: i64, span: Span },
    Var { name: Ident, span: Span },
}

fn main() {}
