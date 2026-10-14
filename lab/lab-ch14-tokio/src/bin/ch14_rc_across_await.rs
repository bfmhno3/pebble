use std::rc::Rc;

#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async {
        let shared = Rc::new(5);
        tokio::task::yield_now().await;
        println!("{shared}");
    });
    handle.await.unwrap();
}
