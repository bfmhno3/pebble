use lab_ch04_stages::{Lexer, Token};
use pebble_span::SourceId;

fn first_token(src: &str) -> Option<Token> {
    Lexer::new(SourceId(0), src).next_token()?.ok()
}

fn main() {
    println!("{:?}", first_token("hello"));
}
