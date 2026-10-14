#![allow(dead_code)]
#[derive(Debug)]
enum E {
    Int(i64),
    Add(Box<E>, Box<E>),
}

fn main() {
    let left = E::Int(1);
    let right = E::Int(2);
    let sum = E::Add(Box::new(left), Box::new(right));
    println!("{sum:?} from {left:?}");
}
