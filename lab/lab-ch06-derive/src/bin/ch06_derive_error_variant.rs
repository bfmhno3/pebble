use lab_macros::Spanned;
use pebble_span::Span;

#[derive(Spanned)]
enum Node {
    Leaf { span: Span },
    Branch { children: u32 },
}

fn main() {}
