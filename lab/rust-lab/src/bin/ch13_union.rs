#[repr(C)]
union Bits {
    float: f32,
    int: u32,
}

fn main() {
    let bits = Bits { float: 1.0 };
    // SAFETY: every bit pattern is a valid u32, and the union was initialised
    // through the f32 field, which fills all four bytes.
    let int = unsafe { bits.int };
    println!("1.0f32 as bits = {int:#010x}");
    println!("size_of::<Bits>() = {}", std::mem::size_of::<Bits>());
}
