// ~/rust-lab/src/bin/ch09_fn_bound.rs
// A cut-down `with_call_depth`: `body` is declared `Fn`, but we move `args` into it.
fn with_depth<T>(body: impl Fn() -> T) -> T {
    body()
}

fn main() {
    let args: Vec<i64> = vec![1, 2, 3];
    let total = with_depth(move || {
        let consumed = args; // moves `args` out of the closure's captured state
        consumed.len()
    });
    println!("{total}");
}
