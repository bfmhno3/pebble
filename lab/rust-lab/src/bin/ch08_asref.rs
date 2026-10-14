fn shout(text: impl AsRef<str>) -> String {
    text.as_ref().to_uppercase()
}

fn main() {
    println!("{}", shout("pebble"));
    println!("{}", shout(String::from("pebble")));
    println!("{}", shout(std::rc::Rc::<str>::from("pebble")));
}
