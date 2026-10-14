use std::fmt;

impl fmt::Display for Vec<u8> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bytes", self.len())
    }
}

fn main() {
    println!("{}", vec![1u8, 2, 3]);
}
