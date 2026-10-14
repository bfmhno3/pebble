use std::time::{Duration, Instant};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let start = Instant::now();

    let ticker = tokio::spawn(async move {
        for _ in 0..3 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            println!("tick at ~{} ms", start.elapsed().as_millis() / 100 * 100);
        }
    });

    let blocker = tokio::spawn(async {
        println!("blocking the only worker thread for 500 ms");
        std::thread::sleep(Duration::from_millis(500));
        println!("blocker done");
    });

    let _ = tokio::join!(ticker, blocker);
}
