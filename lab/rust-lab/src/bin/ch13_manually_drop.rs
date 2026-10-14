use std::mem::ManuallyDrop;

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let _a = Noisy("a: ordinary");
    let _b = ManuallyDrop::new(Noisy("b: manually dropped"));
    let mut c = ManuallyDrop::new(Noisy("c: dropped by hand"));
    println!("before the explicit drop");
    // SAFETY: `c` is never used again after this call.
    unsafe { ManuallyDrop::drop(&mut c) };
    println!("end of main");
}
