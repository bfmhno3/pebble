use std::sync::Arc;
use std::time::{Duration, Instant};

use pebble_net::{ServerConfig, Stats, handle_connection, run_client};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

const FIB: &str = "fn fib(n) { if n < 2 { return n; } return fib(n - 1) + fib(n - 2); } fib(24);";

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let stats = Arc::new(Mutex::new(Stats::default()));

    let accept = {
        let stats = Arc::clone(&stats);
        tokio::spawn(async move {
            for _ in 0..4 {
                let (stream, _) = listener.accept().await.unwrap();
                let stats = Arc::clone(&stats);
                tokio::spawn(handle_connection(stream, ServerConfig::default(), stats));
            }
        })
    };

    let start = Instant::now();
    let (a, b, c, d) = tokio::join!(
        run_client(addr, FIB),
        run_client(addr, "let x = 6 * 7; x;"),
        run_client(addr, "print(\"hello\"); 1;"),
        run_client(addr, "1 + ;"),
    );
    let _elapsed = start.elapsed();
    println!("fib(24)  -> {}", a.unwrap());
    println!("x        -> {}", b.unwrap());
    println!("print    -> {:?}", c.unwrap());
    println!("syntax   -> {}", d.unwrap().lines().next().unwrap());

    accept.await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    println!("{:?}", stats.lock().await);
}
