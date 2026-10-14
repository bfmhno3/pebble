use std::sync::Mutex;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let counter = std::sync::Arc::new(Mutex::new(0u32));
    let task = {
        let counter = counter.clone();
        tokio::spawn(async move {
            let mut guard = counter.lock().unwrap();
            *guard += 1;
            tokio::time::sleep(Duration::from_millis(10)).await;
            *guard += 1;
        })
    };
    task.await.unwrap();
}
