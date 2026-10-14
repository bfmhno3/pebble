fn main() {
    let x = 42u32;
    let p = &x as *const u32;
    // Creating a raw pointer is safe. Reading through it is not.
    println!("{}", unsafe { *p });
}
