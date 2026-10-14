use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);

        let job = tokio::task::spawn_blocking(move || {
            let mut steps = 0u64;
            while !flag.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(10));
                steps += 1;
            }
            println!("the job noticed the flag after {steps} steps and stopped");
        });

        if tokio::time::timeout(Duration::from_millis(200), job).await.is_err() {
            cancel.store(true, Ordering::Relaxed);
            println!("timeout fired, flag set (~{} ms)", start.elapsed().as_millis() / 100 * 100);
        }
    });

    drop(runtime);
    println!("runtime dropped at ~{} ms", start.elapsed().as_millis() / 100 * 100);
}
