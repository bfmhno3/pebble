struct Token<'src> {
    text: &'src str,
}

fn first_word(src: &str) -> Token<'_> {
    Token { text: src.split(' ').next().unwrap_or("") }
}

fn main() {
    let token;
    {
        let source = String::from("let answer = 42");
        token = first_word(&source);
    }
    println!("{}", token.text);
}
