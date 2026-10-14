#![allow(dead_code)]
#[derive(Debug)]
enum Token {
    Jump(u32),
    Name(String),
}

fn main() {
    let mut code = vec![Token::Jump(0), Token::Name("x".to_string())];
    let at = 1;
    code[at] = match code[at] {
        Token::Jump(_) => Token::Jump(7),
        other => panic!("cannot patch {other:?} as a jump"),
    };
    println!("{code:?}");
}
