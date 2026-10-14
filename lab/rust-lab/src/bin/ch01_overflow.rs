fn main() {
    // `black_box` hides the value from the optimizer so the compiler cannot
    // reject the overflow at compile time.
    let big: u8 = std::hint::black_box(255);

    println!("wrapping_add   = {}", big.wrapping_add(1));
    println!("saturating_add = {}", big.saturating_add(1));
    println!("checked_add    = {:?}", big.checked_add(1));
    println!("overflowing    = {:?}", big.overflowing_add(1));

    let plain = big + 1;
    println!("big + 1        = {plain}");
}
