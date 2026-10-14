fn main() {
    let mut files = vec![String::from("a.pebble")];
    let first = &files[0];
    files.push(String::from("b.pebble"));
    println!("first = {first}");
}
