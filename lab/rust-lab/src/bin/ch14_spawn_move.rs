use std::thread;

fn main() {
    let files = vec!["fib.pebble".to_string(), "hello.pebble".to_string()];

    let handle = thread::spawn(move || {
        println!("checking {} files", files.len());
        files.len()
    });

    let n = handle.join().unwrap();
    println!("the thread returned {n}");
}
