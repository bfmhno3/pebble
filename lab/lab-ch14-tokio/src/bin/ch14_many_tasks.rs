use std::time::{Duration, Instant};

const N: usize = 100_000;

#[tokio::main]
async fn main() {
    let start = Instant::now();
    let tasks: Vec<_> = (0..N)
        .map(|i| {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                i
            })
        })
        .collect();

    let mut sum = 0usize;
    for task in tasks {
        sum += task.await.unwrap();
    }
    println!("{N} tasks, sum {sum}, took ~{} ms", start.elapsed().as_millis() / 100 * 100);
    println!("size of one future: {} bytes", {
        let fut = async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            0usize
        };
        std::mem::size_of_val(&fut)
    });
}
