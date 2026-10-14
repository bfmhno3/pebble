use lab_macros_v2::Spanned;
use pebble_span::Span;

#[derive(Spanned)]
enum Expr {
    Int { value: i64, span: Span },
    Nil { span: Span },
}

fn main() {}
