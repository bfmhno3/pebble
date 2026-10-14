use std::mem::size_of;

fn main() {
    let literal: &str = "a.pebble";
    let mut owned: String = String::from("a.pebble");
    owned.push_str(".bak");
    let view: &str = &owned[..8];

    println!("literal = {literal}, owned = {owned}, view = {view}");
    println!("size_of::<String>() = {}", size_of::<String>());
    println!("size_of::<&str>()   = {}", size_of::<&str>());
    println!("len = {}, capacity >= len: {}", owned.len(), owned.capacity() >= owned.len());
}
