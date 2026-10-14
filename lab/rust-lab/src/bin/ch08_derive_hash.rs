#[derive(Hash, PartialEq, Eq)]
enum Key {
    Int(i64),
    Float(f64),
}

fn main() {
    let _ = Key::Int(1);
    let _ = Key::Float(1.0);
}
