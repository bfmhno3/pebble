use lab_macros_attr::Spanned;
use pebble_span::{SourceId, Span, Spanned};

#[derive(Spanned)]
struct Token {
    text: String,
    #[spanned]
    range: Span,
}

fn main() {
    let token = Token {
        text: "let".to_string(),
        range: Span::new(SourceId(0), 0, 3),
    };
    println!("{} -> {:?}", token.text, token.span());
}
