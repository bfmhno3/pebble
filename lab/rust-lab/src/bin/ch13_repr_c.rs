use std::mem::{align_of, size_of};

#[allow(dead_code)]
struct Rusty {
    a: u8,
    b: u32,
    c: u8,
}

#[allow(dead_code)]
#[repr(C)]
struct CLike {
    a: u8,
    b: u32,
    c: u8,
}

fn main() {
    println!("Rusty: size {} align {}", size_of::<Rusty>(), align_of::<Rusty>());
    println!("CLike: size {} align {}", size_of::<CLike>(), align_of::<CLike>());
}
