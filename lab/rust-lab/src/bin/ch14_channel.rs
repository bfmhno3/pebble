use std::sync::mpsc;
use std::thread;

fn main() {
    let (sender, receiver) = mpsc::channel::<String>();

    let workers: Vec<_> = ["fib", "hello", "errors"]
        .into_iter()
        .map(|name| {
            let sender = sender.clone();
            thread::spawn(move || {
                sender.send(format!("checked {name}.pebble")).unwrap();
            })
        })
        .collect();
    drop(sender);

    let mut reports: Vec<String> = receiver.iter().collect();
    reports.sort();
    for report in &reports {
        println!("{report}");
    }

    for worker in workers {
        worker.join().unwrap();
    }
    println!("{} reports, channel closed", reports.len());
}
