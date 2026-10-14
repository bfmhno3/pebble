use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    let chained: Vec<u32> = thread::scope(|scope| {
        (0..4u32)
            .map(|n| {
                scope.spawn(move || {
                    thread::sleep(Duration::from_millis(100));
                    n
                })
            })
            .map(|handle| handle.join().unwrap())
            .collect()
    });
    println!("chained spawn+join:    {chained:?} in ~{} ms", start.elapsed().as_millis() / 100 * 100);

    let start = Instant::now();
    let collected: Vec<u32> = thread::scope(|scope| {
        let handles: Vec<_> = (0..4u32)
            .map(|n| {
                scope.spawn(move || {
                    thread::sleep(Duration::from_millis(100));
                    n
                })
            })
            .collect();
        handles.into_iter().map(|handle| handle.join().unwrap()).collect()
    });
    println!("collect, then join:    {collected:?} in ~{} ms", start.elapsed().as_millis() / 100 * 100);
}
