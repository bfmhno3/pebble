use std::cell::Cell;
use std::thread;

fn main() {
    let hits = Cell::new(0u32);

    thread::scope(|scope| {
        scope.spawn(|| hits.set(hits.get() + 1));
        scope.spawn(|| hits.set(hits.get() + 1));
    });

    println!("{}", hits.get());
}
