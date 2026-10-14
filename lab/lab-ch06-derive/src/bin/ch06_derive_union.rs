use lab_macros::Spanned;
use pebble_span::Span;

#[derive(Spanned)]
union Raw {
    span: Span,
    bits: u64,
}

fn main() {}
