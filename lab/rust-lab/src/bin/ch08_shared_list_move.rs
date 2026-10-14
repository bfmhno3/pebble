fn main() {
    let a: Vec<i64> = Vec::new();
    let b = a;
    let mut b = b;
    b.push(1);
    println!("{:?}", a);
}
