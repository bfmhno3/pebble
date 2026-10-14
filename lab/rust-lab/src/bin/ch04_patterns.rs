fn classify(ch: char) -> &'static str {
    match ch {
        '0'..='9' => "digit",
        '_' | 'a'..='z' | 'A'..='Z' => "ident start",
        c if c.is_alphabetic() => "unicode letter (guard)",
        c if c.is_whitespace() => "space",
        _ => "other",
    }
}

fn main() {
    for ch in ['7', '_', 'x', '中', ' ', '@'] {
        println!("{ch:?} -> {}", classify(ch));
    }
    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() {
        print!("{top} ");
    }
    println!();
    println!("{:?}", "123".parse::<i64>());
    println!("{:?}", "99999999999999999999".parse::<i64>());
    println!("{:?}", "99999999999999999999".parse::<f64>());
    println!("{:?}", "1.5".parse::<i64>());
    println!("'a'.len_utf8() = {}, '中'.len_utf8() = {}, '🦀'.len_utf8() = {}", 'a'.len_utf8(), '中'.len_utf8(), '🦀'.len_utf8());
}
