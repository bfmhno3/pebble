fn main() {
    let offset = 10;
    let add_offset = move |x: i64| x + offset;
    let pointer: fn(i64) -> i64 = add_offset;
    println!("{}", pointer(1));
}
