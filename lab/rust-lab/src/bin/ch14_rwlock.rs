use std::sync::{Arc, RwLock};
use std::thread;

fn main() {
    let table = Arc::new(RwLock::new(vec![1, 2, 3]));

    let readers: Vec<_> = (0..3)
        .map(|_| {
            let table = Arc::clone(&table);
            thread::spawn(move || table.read().unwrap().len())
        })
        .collect();

    table.write().unwrap().push(4);

    let lengths: Vec<usize> = readers.into_iter().map(|h| h.join().unwrap()).collect();
    println!("every reader saw 3 or 4 elements: {}", lengths.iter().all(|n| (3..=4).contains(n)));
    println!("final {:?}", table.read().unwrap());
}
