use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let job = tokio::task::spawn_blocking(|| {
            std::thread::sleep(Duration::from_secs(2));
            println!("the blocking job finally finished");
        });
        let result = tokio::time::timeout(Duration::from_millis(200), job).await;
        println!(
            "timeout fired: {} (after ~{} ms)",
            result.is_err(),
            start.elapsed().as_millis() / 100 * 100
        );
    });

    println!("block_on returned at ~{} ms", start.elapsed().as_millis() / 100 * 100);
    drop(runtime);
    println!("runtime dropped at ~{} ms", start.elapsed().as_millis() / 100 * 100);
}
