use std::sync::Mutex;

fn main() {
    let counter = Mutex::new(0u32);
    let guard: u32 = counter.lock().unwrap();
    println!("{guard}");
}
