use pebble_sys::Arena;

fn main() {
    let mut arena = Arena::with_capacity(64).expect("no memory");
    for len in [1, 1, 24] {
        let slice = arena.alloc(len).expect("full");
        println!("len {len:>2} at address ...{:02x}", slice.as_ptr() as usize & 0xff);
        println!("   used = {}", arena.used());
    }
    println!("a fourth, 8-byte request: {:?}", arena.alloc(8));
}
