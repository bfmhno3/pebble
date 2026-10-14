use std::fmt;

struct Oops;

impl fmt::Display for Oops {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("oops")
    }
}

impl std::error::Error for Oops {}

fn main() {
    let error: Box<dyn std::error::Error> = Box::new(Oops);
    println!("{error}");
}
