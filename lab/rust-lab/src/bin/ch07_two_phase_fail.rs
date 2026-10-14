fn main() {
    let mut v = vec![1, 2, 3];
    Vec::push(&mut v, v.len());
    println!("{v:?}");
}
