fn main() {
    let x = 42u32;
    let p = &x as *const u32;
    println!("{}", *p);
}
