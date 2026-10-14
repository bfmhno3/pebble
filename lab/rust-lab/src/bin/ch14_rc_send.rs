use std::rc::Rc;
use std::thread;

fn main() {
    let shared = Rc::new(5);
    let clone = Rc::clone(&shared);

    let handle = thread::spawn(move || {
        println!("in the thread: {clone}");
    });

    handle.join().unwrap();
    println!("in main: {shared}");
}
