use std::thread;

fn main() {
    let mut counter = 0u32;

    thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| {
                for _ in 0..1000 {
                    counter += 1;
                }
            });
        }
    });

    println!("{counter}");
}
