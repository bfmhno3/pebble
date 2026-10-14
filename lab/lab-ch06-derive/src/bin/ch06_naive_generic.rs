use lab_macros_step::SpannedStruct;
use pebble_span::Span;

#[derive(SpannedStruct)]
struct Tagged<T> {
    inner: T,
    span: Span,
}

fn main() {}
