use lab_macros_step::Show;
use pebble_span::Span;

#[derive(Show)]
struct Ident {
    name: String,
    span: Span,
}

fn main() {}
