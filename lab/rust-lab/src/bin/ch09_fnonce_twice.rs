// ~/rust-lab/src/bin/ch09_fnonce_twice.rs
fn main() {
    let name = String::from("pebble");
    let consume = move || name; // returns the captured String: it can only run once
    let first = consume();
    let second = consume();
    println!("{first} {second}");
}
