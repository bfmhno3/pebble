use std::mem::{size_of, MaybeUninit};

fn main() {
    println!("size_of::<u8>()               = {}", size_of::<u8>());
    println!("size_of::<MaybeUninit<u8>>()  = {}", size_of::<MaybeUninit<u8>>());
    println!("size_of::<MaybeUninit<u64>>() = {}", size_of::<MaybeUninit<u64>>());

    // A buffer that does not claim to contain valid bytes yet.
    let mut buffer: Vec<MaybeUninit<u8>> = vec![MaybeUninit::uninit(); 4];
    buffer[0].write(b'o');
    buffer[1].write(b'k');
    // SAFETY: indices 0 and 1 were initialised on the two lines above.
    let ok = unsafe { [buffer[0].assume_init(), buffer[1].assume_init()] };
    println!("{:?}", std::str::from_utf8(&ok));
}
