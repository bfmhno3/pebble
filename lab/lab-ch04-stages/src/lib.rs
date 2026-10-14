use pebble_diag::Diagnostic;
use pebble_span::{SourceId, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind<'src> {
    Ident(&'src str),
    Int(i64),
    LParen,
    RParen,
    Plus,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token<'src> {
    pub kind: TokenKind<'src>,
    pub span: Span,
}

pub struct Lexer<'src> {
    source: SourceId,
    src: &'src str,
    pos: usize,
}

impl<'src> Lexer<'src> {
    pub const fn new(source: SourceId, src: &'src str) -> Self {
        Self { source, src, pos: 0 }
    }

    fn rest(&self) -> &'src str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn span_from(&self, start: usize) -> Span {
        Span::new(self.source, start as u32, self.pos as u32)
    }

    pub fn next_token(&mut self) -> Option<Result<Token<'src>, Diagnostic>> {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
        let start = self.pos;
        let ch = self.bump()?;
        let kind = match ch {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '+' => TokenKind::Plus,
            '0'..='9' => {
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.bump();
                }
                TokenKind::Int(self.src[start..self.pos].parse().unwrap())
            }
            ch if ch == '_' || ch.is_alphabetic() => {
                while self.peek().is_some_and(|c| c == '_' || c.is_alphanumeric()) {
                    self.bump();
                }
                TokenKind::Ident(&self.src[start..self.pos])
            }
            other => {
                return Some(Err(Diagnostic::error(format!("unexpected character `{other}`"))
                    .with_span(self.span_from(start))));
            }
        };
        Some(Ok(Token { kind, span: self.span_from(start) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_borrow_the_source() {
        let source = String::from("hello");
        let mut lexer = Lexer::new(SourceId(0), &source);
        let token = lexer.next_token().unwrap().unwrap();
        match token.kind {
            TokenKind::Ident(name) => {
                let base = source.as_ptr() as usize;
                let borrowed = name.as_ptr() as usize;
                assert!((base..base + source.len()).contains(&borrowed));
            }
            other => panic!("expected identifier, got {other:?}"),
        }
    }

    #[test]
    fn a_token_outlives_its_lexer() {
        let source = String::from("answer + 42");
        let first = {
            let mut lexer = Lexer::new(SourceId(0), &source);
            lexer.next_token().unwrap().unwrap()
        };
        assert_eq!(first.kind, TokenKind::Ident("answer"));
    }
}
