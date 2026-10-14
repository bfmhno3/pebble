fn main() {
    let mut a: Vec<i64> = Vec::new();
    let b = &mut a;
    let c = &mut a;
    b.push(1);
    c.push(2);
    println!("{:?}", a);
}
