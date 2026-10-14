fn main() {
    let a = 0.0_f64;
    let b = -0.0_f64;
    let nan = f64::NAN;
    println!("0.0 == -0.0        -> {}", a == b);
    println!("NaN == NaN         -> {}", nan == nan);
    println!("bits equal 0/-0    -> {}", a.to_bits() == b.to_bits());
    println!("bits equal NaN/NaN -> {}", nan.to_bits() == nan.to_bits());
}
