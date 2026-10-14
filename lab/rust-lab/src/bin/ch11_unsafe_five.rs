// The five things `unsafe` unlocks, one line each.
static mut COUNTER: u32 = 0;

union IntOrFloat {
    i: u32,
    f: f32,
}

unsafe trait Zeroable {}
unsafe impl Zeroable for u32 {}

unsafe fn first_byte(p: *const u8) -> u8 {
    unsafe { *p }
}

fn needs_zeroable<T: Zeroable>() {}

fn main() {
    needs_zeroable::<u32>();
    let bytes = [7u8, 8, 9];
    unsafe {
        // 1. dereference a raw pointer
        println!("deref: {}", *bytes.as_ptr());
        // 2. call an unsafe function
        println!("call: {}", first_byte(bytes.as_ptr()));
        // 3. read or write a mutable static
        COUNTER += 1;
        println!("static mut: {}", *(&raw const COUNTER));
        // 4. (unsafe trait: see the `unsafe impl` above)
        // 5. read a union field
        let u = IntOrFloat { f: 1.0 };
        println!("union: {:#x}", u.i);
    }
}
