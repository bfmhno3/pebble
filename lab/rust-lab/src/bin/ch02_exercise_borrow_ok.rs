fn main() {
    let mut names = vec![String::from("a.pebble")];
    names.push(String::from("b.pebble"));
    let newest = names.last().unwrap();
    println!("{newest}");
}
