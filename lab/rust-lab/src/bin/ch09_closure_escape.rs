// ~/rust-lab/src/bin/ch09_closure_escape.rs
fn make_counter() -> impl FnMut() -> i32 {
    let mut n = 0;
    || {
        n += 1;
        n
    }
}

fn main() {
    let mut counter = make_counter();
    println!("{} {}", counter(), counter());
}
