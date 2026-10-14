#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
    Int(i64),
    Float(f64),
}

fn main() {
    println!("{:?}", TokenKind::Float(1.5));
}
