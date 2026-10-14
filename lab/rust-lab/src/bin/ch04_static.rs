fn keyword(word: &str) -> Option<&'static str> {
    match word {
        "let" => Some("Let"),
        "fn" => Some("Fn"),
        _ => None,
    }
}

fn main() {
    let result;
    {
        let owned = String::from("let");
        result = keyword(&owned);
    }
    println!("{result:?}");
}
