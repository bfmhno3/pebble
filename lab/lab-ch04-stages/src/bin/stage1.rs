use lab_ch04_stages::Lexer;
use pebble_span::SourceId;

fn main() {
    let mut lexer = Lexer::new(SourceId(0), "total + (x1 + 42)");
    while let Some(item) = lexer.next_token() {
        println!("{item:?}");
    }
    let mut lexer = Lexer::new(SourceId(0), "a @ b");
    while let Some(item) = lexer.next_token() {
        match item {
            Ok(token) => println!("ok  {:?}", token.kind),
            Err(diagnostic) => println!("err {}", diagnostic.message),
        }
    }
}
