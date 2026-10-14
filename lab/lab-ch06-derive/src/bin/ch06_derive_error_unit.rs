use lab_macros::Spanned;
use pebble_span::Span;

#[derive(Spanned)]
enum Node {
    Leaf { span: Span },
    Empty,
}

fn main() {}
