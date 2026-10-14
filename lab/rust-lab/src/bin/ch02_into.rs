fn register(name: impl Into<String>) -> usize {
    let owned: String = name.into();
    owned.len()
}

fn main() {
    let literal = register("a.pebble");
    let owned = register(String::from("bb.pebble"));
    let formatted = register(format!("{}.pebble", 3));
    println!("{literal} {owned} {formatted}");
}
