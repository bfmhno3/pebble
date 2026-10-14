use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let data = Arc::new(Mutex::new(vec![1, 2, 3]));

    let worker = {
        let data = Arc::clone(&data);
        thread::spawn(move || {
            let mut guard = data.lock().unwrap();
            guard.push(4);
            panic!("worker died while holding the lock");
        })
    };
    let _ = worker.join();

    println!("is_poisoned = {}", data.is_poisoned());
    match data.lock() {
        Ok(guard) => println!("clean: {guard:?}"),
        Err(poisoned) => {
            let guard = poisoned.into_inner();
            println!("recovered: {guard:?}");
        }
    }
}
