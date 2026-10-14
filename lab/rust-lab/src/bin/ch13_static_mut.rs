static mut COUNTER: u32 = 0;

fn bump() -> u32 {
    unsafe {
        COUNTER += 1;
        let r = &COUNTER;
        *r
    }
}

fn main() {
    println!("{}", bump());
}
