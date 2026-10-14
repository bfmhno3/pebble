use lab_macros::Spanned;
use pebble_span::Span;

#[derive(Spanned)]
struct Pair(Span, u32);

fn main() {}
