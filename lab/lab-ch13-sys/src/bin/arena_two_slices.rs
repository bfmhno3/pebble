use pebble_sys::Arena;

fn main() {
    let mut arena = Arena::with_capacity(64).expect("no memory");
    let a = arena.alloc(8).expect("full");
    let b = arena.alloc(8).expect("full");
    a[0] = 1;
    b[0] = 2;
    println!("{} {}", a[0], b[0]);
}
