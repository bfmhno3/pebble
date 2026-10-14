fn main() {
    let mut names = vec![String::from("a.pebble")];
    let newest = names.last().unwrap();
    names.push(String::from("b.pebble"));
    println!("{newest}");
}
