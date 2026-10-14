use std::fmt;

struct Pt(i32, i32);

impl fmt::Display for Pt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

fn main() {
    // We never wrote `to_string` for Pt.
    let s: String = Pt(1, 2).to_string();
    println!("{s}");
    println!("{}", 42.to_string() + &"x".to_string());
}
