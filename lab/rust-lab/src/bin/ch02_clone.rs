fn byte_len(text: String) -> usize {
    text.len()
}

fn main() {
    let name = String::from("a.pebble");
    let n = byte_len(name.clone());
    println!("{n} bytes in {name}");

    let moved_to = name;
    println!("{moved_to}");
}
