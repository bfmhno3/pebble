fn byte_len(text: String) -> usize {
    text.len()
}

fn main() {
    let name = String::from("a.pebble");
    let n = byte_len(name);
    println!("{n} bytes in {name}");
}
