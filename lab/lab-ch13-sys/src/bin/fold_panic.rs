use std::panic::catch_unwind;

fn main() {
    let result = catch_unwind(|| {
        pebble_sys::fold(0, 1..=5, |acc, value| {
            assert_ne!(value, 3, "closure exploded");
            acc + value
        })
    });
    match result {
        Ok(total) => println!("total = {total}"),
        Err(payload) => {
            let message = payload.downcast_ref::<String>().map(String::as_str).unwrap_or("?");
            println!("caught on the Rust side: {message}");
        }
    }
}
