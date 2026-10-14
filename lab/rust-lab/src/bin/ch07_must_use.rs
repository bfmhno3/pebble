fn parse_digit(c: char) -> Result<u32, String> {
    c.to_digit(10).ok_or(format!("`{c}` is not a digit"))
}

fn main() {
    parse_digit('x');
    println!("done");
}
