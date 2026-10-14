#[inline]
pub fn small(x: u32) -> u32 {
    x.wrapping_mul(31).wrapping_add(7)
}

#[inline(always)]
pub fn tiny(x: u32) -> u32 {
    x ^ 0x9e37_79b9
}

#[inline(never)]
pub fn big(x: u32) -> u32 {
    (0..x % 8).fold(x, |acc, i| small(acc ^ i))
}

fn main() {
    println!("{} {} {}", small(1), tiny(1), big(21));
}
