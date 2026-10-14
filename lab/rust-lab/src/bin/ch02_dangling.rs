fn file_name() -> &str {
    let name = String::from("a.pebble");
    &name
}

fn main() {
    println!("{}", file_name());
}
