use std::rc::Rc;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let handle = tokio::task::spawn_local(async {
                let shared = Rc::new(5);
                tokio::task::yield_now().await;
                println!("an Rc across an await: {shared}");
            });
            handle.await.unwrap();
        })
        .await;
}
