use lab_macros::Spanned;

struct Span;

#[derive(Spanned)]
struct Ident {
    span: Span,
}

fn main() {}
