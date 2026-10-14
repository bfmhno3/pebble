use pebble_sys::Arena;
use std::thread;

fn main() {
    let arena = Arena::with_capacity(64).expect("no memory");
    let handle = thread::spawn(move || arena.used());
    println!("used = {}", handle.join().unwrap());
}
