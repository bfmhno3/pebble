use std::ops::Add;
use std::rc::Rc;

#[derive(Clone, Debug)]
enum Value {
    Int(i64),
    Str(Rc<str>),
}

impl Add for Value {
    type Output = Result<Value, String>;

    fn add(self, rhs: Value) -> Self::Output {
        match (self, rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}").into())),
            (a, b) => Err(format!("cannot add {a:?} and {b:?}")),
        }
    }
}

fn main() {
    let a = Value::Int(1);
    let b = Value::Int(2);
    println!("{:?}", a.clone() + b.clone());
    println!("{:?}", a.clone() + Value::Str("x".into()));
    let sum = a + b;
    println!("{:?}", sum);
}
