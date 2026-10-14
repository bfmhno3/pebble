use std::panic::catch_unwind;

fn main() {
    let mut total = 0;
    let result = catch_unwind(|| {
        total += 1;
        panic!("boom");
    });
    println!("{result:?} {total}");
}
