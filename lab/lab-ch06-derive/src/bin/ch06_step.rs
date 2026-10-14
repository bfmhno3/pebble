use lab_macros_step::{Hello, SpannedStruct};
use pebble_span::{SourceId, Span, Spanned};

#[derive(Hello)]
struct Anything;

#[derive(SpannedStruct)]
struct Ident {
    name: String,
    span: Span,
}

fn main() {
    println!("{}", Anything::hello());
    let ident = Ident { name: "x".to_string(), span: Span::new(SourceId(0), 4, 5) };
    println!("{} at {:?}", ident.name, ident.span());
}
