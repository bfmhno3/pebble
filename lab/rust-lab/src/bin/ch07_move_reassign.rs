#![allow(dead_code)]
#[derive(Debug)]
enum E {
    Int(i64),
    Add(Box<E>, Box<E>),
}

fn main() {
    let mut left = E::Int(1);
    for n in 2..=4 {
        let right = E::Int(n);
        // `left` moves into the Box, then the assignment gives it a new value.
        left = E::Add(Box::new(left), Box::new(right));
    }
    println!("{left:?}");
}
