struct Lexer<'src> {
    src: &'src str,
    pos: usize,
}

impl<'src> Lexer<'src> {
    // Named: the result borrows the source, not the lexer.
    fn next_word(&mut self) -> &'src str {
        let rest = &self.src[self.pos..];
        let end = rest.find(' ').unwrap_or(rest.len());
        self.pos += (end + 1).min(rest.len());
        &rest[..end]
    }
}

fn main() {
    let mut lexer = Lexer { src: "let answer", pos: 0 };
    let a = lexer.next_word();
    let b = lexer.next_word();
    println!("{a} {b}");
}
