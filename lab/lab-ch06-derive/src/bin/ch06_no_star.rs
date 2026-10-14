use lab_macros_nostar::Spanned;
use pebble_span::Span;

#[derive(Spanned)]
enum Expr {
    Int { value: i64, span: Span },
    Nil { span: Span },
}

fn main() {}
