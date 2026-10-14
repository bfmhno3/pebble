struct Token {
    text: &str,
}

fn main() {
    let t = Token { text: "let" };
    println!("{}", t.text);
}
