use std::thread;

fn main() {
    let files = vec!["fib.pebble".to_string(), "hello.pebble".to_string()];

    let lengths: Vec<usize> = thread::scope(|scope| {
        let handles: Vec<_> = files
            .iter()
            .map(|name| scope.spawn(|| name.len()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    println!("{lengths:?}");
    println!("still own {} files", files.len());
}
