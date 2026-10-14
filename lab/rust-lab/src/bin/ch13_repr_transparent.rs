use std::mem::{align_of, size_of};

#[repr(transparent)]
struct Handle(u64);

#[allow(dead_code)]
struct Plain(u64, ());

unsafe extern "C" {
    fn abs(x: i32) -> i32;
}

fn main() {
    println!("Handle: size {} align {}", size_of::<Handle>(), align_of::<Handle>());
    println!("Plain:  size {} align {}", size_of::<Plain>(), align_of::<Plain>());
    let h = Handle(7);
    println!("inner = {}", h.0);
    // SAFETY: abs has no preconditions beyond i32::MIN overflow.
    println!("abs(-5) = {}", unsafe { abs(-5) });
}
