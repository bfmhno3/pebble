fn main() {
    let mut files = vec![String::from("a.pebble")];
    let first = &files[0];
    println!("first = {first}");
    files.push(String::from("b.pebble"));
    println!("now {} files", files.len());
}
