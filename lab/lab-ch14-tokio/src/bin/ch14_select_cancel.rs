use std::time::Duration;
use tokio::time::sleep;

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("{} dropped", self.0);
    }
}

async fn slow() -> &'static str {
    let _guard = Noisy("slow's local");
    sleep(Duration::from_millis(500)).await;
    println!("slow finished");
    "slow"
}

async fn quick() -> &'static str {
    sleep(Duration::from_millis(50)).await;
    "quick"
}

#[tokio::main]
async fn main() {
    tokio::select! {
        winner = quick() => println!("winner: {winner}"),
        winner = slow() => println!("winner: {winner}"),
    }
    sleep(Duration::from_millis(600)).await;
    println!("main done; \"slow finished\" never printed");
}
