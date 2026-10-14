use std::ops::Add;

#[derive(Debug)]
struct Value(i64);

impl Add for Value {
    type Output = Value;

    fn add(self, rhs: Value) -> Value {
        Value(self.0 + rhs.0)
    }
}

fn main() {
    let a = Value(1);
    let b = Value(2);
    let sum = a + b;
    println!("{:?} {:?}", sum, a);
}
