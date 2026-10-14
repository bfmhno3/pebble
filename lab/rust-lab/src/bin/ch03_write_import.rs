use std::fmt;

fn header(out: &mut String, code: u32) -> fmt::Result {
    write!(out, "error[E{code:04}]")
}

fn main() {
    let mut out = String::new();
    header(&mut out, 1).unwrap();
    println!("{out}");
}
