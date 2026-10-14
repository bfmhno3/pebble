use std::time::{Duration, Instant};
use tokio::time::sleep;

async fn check(name: &str, millis: u64) -> String {
    sleep(Duration::from_millis(millis)).await;
    format!("checked {name}")
}

#[tokio::main]
async fn main() {
    let start = Instant::now();
    let a = check("fib", 200).await;
    let b = check("hello", 200).await;
    println!("{a}, {b}: sequential took {} ms", start.elapsed().as_millis() / 50 * 50);

    let start = Instant::now();
    let (a, b) = tokio::join!(check("fib", 200), check("hello", 200));
    println!("{a}, {b}: join! took {} ms", start.elapsed().as_millis() / 50 * 50);
}
