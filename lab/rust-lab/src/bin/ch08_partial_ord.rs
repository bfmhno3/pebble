use std::rc::Rc;

#[derive(PartialEq, PartialOrd, Debug)]
enum Naive {
    Int(i64),
    Float(f64),
}

#[derive(Debug)]
struct List(Rc<Vec<i64>>);

impl PartialEq for List {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl PartialOrd for List {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

fn main() {
    println!("Int(2) < Float(0.5): {}", Naive::Int(2) < Naive::Float(0.5));
    println!("Float(NaN) < Float(1.0): {}", Naive::Float(f64::NAN) < Naive::Float(1.0));
    println!("Float(NaN) == Float(NaN): {}", Naive::Float(f64::NAN) == Naive::Float(f64::NAN));

    let a = List(Rc::new(vec![1]));
    let b = List(Rc::new(vec![1]));
    println!("a == b: {}", a == b);
    println!("a.partial_cmp(&b): {:?}", a.partial_cmp(&b));
    println!("a <= b && a >= b: {}", a <= b && a >= b);
}
