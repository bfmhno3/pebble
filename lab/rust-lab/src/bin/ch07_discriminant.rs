use std::mem::discriminant;

#[derive(Debug, PartialEq)]
enum Tok<'a> {
    Ident(&'a str),
    Int(i64),
    Semi,
}

fn main() {
    let wanted = Tok::Ident("");
    let found = Tok::Ident("total");
    println!("== : {}", wanted == found);
    println!("discriminant ==: {}", discriminant(&wanted) == discriminant(&found));
    println!("different variant: {}", discriminant(&wanted) == discriminant(&Tok::Int(1)));
    println!("{:?}", Tok::Semi);
}
