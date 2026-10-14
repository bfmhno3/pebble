#[derive(Debug)]
enum Token {
    Jump(u32),
    Name(String),
}

fn main() {
    let mut code = vec![Token::Jump(0), Token::Name("x".to_string())];
    code[0] = match code[0] {
        Token::Jump(_) => Token::Jump(7),
        ref other => panic!("cannot patch {other:?} as a jump"),
    };
    println!("{code:?}");
}
