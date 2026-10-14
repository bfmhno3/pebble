use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

static REQUESTS: AtomicU64 = AtomicU64::new(0);

fn main() {
    thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for _ in 0..1000 {
                    REQUESTS.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });
    println!("requests = {}", REQUESTS.load(Ordering::Relaxed));
}
