use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
enum TokenKind<'src> {
    Ident(&'src str),
    Str(Cow<'src, str>),
}

fn main() {
    println!("{:?}", TokenKind::Ident("x"));
}
