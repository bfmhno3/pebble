use lab_macros_rel::Spanned;

// A local module with the same name as the real crate.
#[allow(dead_code)]
mod pebble_span {
    pub struct Span;
}

#[derive(Spanned)]
struct Ident {
    span: ::pebble_span::Span,
}

fn main() {
    let ident = Ident { span: ::pebble_span::Span::new(::pebble_span::SourceId(0), 1, 4) };
    println!("{:?}", ::pebble_span::Spanned::span(&ident));
}
