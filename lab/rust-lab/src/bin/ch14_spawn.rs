use std::thread;

fn main() {
    let files = vec!["fib.pebble".to_string(), "hello.pebble".to_string()];

    let handle = thread::spawn(|| {
        println!("checking {} files", files.len());
    });

    handle.join().unwrap();
}
