use std::pin::Pin;

async fn answer() -> u32 {
    let local = [1u32, 2, 3];
    let reference = &local[0];
    std::future::ready(()).await;
    *reference
}

fn main() {
    let mut future = answer();
    let _pinned = Pin::new(&mut future);
}
