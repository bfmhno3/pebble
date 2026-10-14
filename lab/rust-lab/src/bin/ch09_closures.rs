// ~/rust-lab/src/bin/ch09_closures.rs
fn make_counter() -> impl FnMut() -> i32 {
    let mut n = 0;
    move || {
        n += 1;
        n
    }
}

fn main() {
    // 1. A closure that only reads what it captured: Fn.
    let greeting = String::from("hello");
    let shout = || greeting.to_uppercase();
    println!("{} {}", shout(), shout());
    println!("greeting is still ours: {greeting}");

    // 2. A closure that mutates what it captured: FnMut.
    let mut count = 0;
    let mut bump = || {
        count += 1;
        count
    };
    bump();
    bump();
    println!("count after two bumps: {count}");

    // 3. `move` copies the captured value into the closure.
    let mut n = 10;
    let mut owned = move || {
        n += 1;
        n
    };
    let first = owned();
    let second = owned();
    println!("owned() = {first}, then {second}");
    println!("the outer n is untouched: {n}");

    // 4. A function can hand out a closure if the closure owns its state.
    let mut counter = make_counter();
    println!("counter: {} {} {}", counter(), counter(), counter());
}
