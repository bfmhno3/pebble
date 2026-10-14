fn first_word(text: String) -> &str {
    match text.find(' ') {
        Some(end) => &text[..end],
        None => &text,
    }
}

fn main() {
    println!("{}", first_word(String::from("let x = 1;")));
}
