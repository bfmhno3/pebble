fn first_word(text: &str) -> &str {
    match text.find(' ') {
        Some(end) => &text[..end],
        None => text,
    }
}

fn main() {
    let line = String::from("let x = 1;");
    println!("{}", first_word(&line));
}
