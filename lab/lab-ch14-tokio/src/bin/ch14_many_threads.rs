use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let start = Instant::now();
    let mut handles = Vec::new();
    for i in 0..n {
        match thread::Builder::new().spawn(move || {
            thread::sleep(Duration::from_millis(500));
            i
        }) {
            Ok(handle) => handles.push(handle),
            Err(error) => {
                println!("spawn failed after {} threads: {error}", handles.len());
                break;
            }
        }
    }
    let started = handles.len();
    let sum: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    println!("{started} threads, sum {sum}, took ~{} ms", start.elapsed().as_millis() / 100 * 100);
}
