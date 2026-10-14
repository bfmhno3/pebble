fn shout(text: &str) -> &str {
    let loud = text.to_uppercase();
    &loud
}

fn main() {
    println!("{}", shout("pebble"));
}
