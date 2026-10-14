use pebble_interp::Interpreter;

#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async {
        let interpreter = Interpreter::new();
        tokio::task::yield_now().await;
        drop(interpreter);
    });
    handle.await.unwrap();
}
