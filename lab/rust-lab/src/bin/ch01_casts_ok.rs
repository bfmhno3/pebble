fn main() {
    let a: u32 = 7;
    let b: usize = 3;
    println!("{}", a as usize + b);
    println!("{}", a + b as u32);

    // `as` never fails and never warns: it truncates or saturates silently.
    println!("{}", 300_i32 as u8);
    println!("{}", -1_i32 as u32);
    println!("{}", 3.9_f64 as i32);
    println!("{}", -1.0_f64 as u8);
    println!("{}", u32::try_from(300_i64).unwrap());
    println!("{:?}", u8::try_from(300_i32));
}
