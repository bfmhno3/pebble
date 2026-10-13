#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! A hand-written, zero-copy lexer for Pebble.
//!
//! Two design decisions shape this crate:
//!
//! 1. **Zero-copy.** An identifier token does not own a `String`; it borrows a
//!    slice of the original source (`&'src str`). The token stream is therefore
//!    a set of views into the input, and lexing allocates nothing except in the
//!    rare case of a string literal that contains escape sequences.
//! 2. **It keeps going.** A bad character produces a [`Diagnostic`] and the
//!    lexer resumes at the next character, so one run can report every typo in
//!    a file instead of just the first.

extern crate alloc;

use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use pebble_diag::Diagnostic;
use pebble_span::{SourceId, Span};

/// The kind of a token, plus any payload it carries.
///
/// The `&'src str` payloads tie a token to the source string it came from: a
/// [`Token`] may outlive the [`Lexer`] but never the input, which is exactly
/// the guarantee we want. `Clone` (but not `Copy`) because [`Cow`] may own data.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind<'src> {
    // --- literals -----------------------------------------------------------
    /// `123`
    Int(i64),
    /// `1.5`, `0.25`
    Float(f64),
    /// `"..."`, with escapes already resolved.
    Str(Cow<'src, str>),
    /// `true`
    True,
    /// `false`
    False,
    /// `nil`
    Nil,

    // --- identifiers and keywords ------------------------------------------
    /// `foo`, `_bar`
    Ident(&'src str),
    /// `let`
    Let,
    /// `fn`
    Fn,
    /// `if`
    If,
    /// `else`
    Else,
    /// `while`
    While,
    /// `for`
    For,
    /// `in`
    In,
    /// `return`
    Return,
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `and` or `&&`
    And,
    /// `or` or `||`
    Or,
    /// `not` or `!`
    Not,

    // --- punctuation --------------------------------------------------------
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `#{`, the start of a map literal
    HashBrace,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `=`
    Assign,
    /// `==`
    Eq,
    /// `!=`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,

    /// Synthetic token emitted once at the end of input.
    Eof,
}

impl TokenKind<'_> {
    /// A short human description used in parser error messages.
    #[must_use]
    pub const fn describe(&self) -> &'static str {
        match self {
            TokenKind::Int(_) => "integer",
            TokenKind::Float(_) => "float",
            TokenKind::Str(_) => "string",
            TokenKind::True | TokenKind::False => "boolean",
            TokenKind::Nil => "`nil`",
            TokenKind::Ident(_) => "identifier",
            TokenKind::Let => "`let`",
            TokenKind::Fn => "`fn`",
            TokenKind::If => "`if`",
            TokenKind::Else => "`else`",
            TokenKind::While => "`while`",
            TokenKind::For => "`for`",
            TokenKind::In => "`in`",
            TokenKind::Return => "`return`",
            TokenKind::Break => "`break`",
            TokenKind::Continue => "`continue`",
            TokenKind::And => "`and`",
            TokenKind::Or => "`or`",
            TokenKind::Not => "`not`",
            TokenKind::LParen => "`(`",
            TokenKind::RParen => "`)`",
            TokenKind::LBrace => "`{`",
            TokenKind::RBrace => "`}`",
            TokenKind::LBracket => "`[`",
            TokenKind::RBracket => "`]`",
            TokenKind::HashBrace => "`#{`",
            TokenKind::Comma => "`,`",
            TokenKind::Colon => "`:`",
            TokenKind::Semicolon => "`;`",
            TokenKind::Plus => "`+`",
            TokenKind::Minus => "`-`",
            TokenKind::Star => "`*`",
            TokenKind::Slash => "`/`",
            TokenKind::Percent => "`%`",
            TokenKind::Assign => "`=`",
            TokenKind::Eq => "`==`",
            TokenKind::NotEq => "`!=`",
            TokenKind::Lt => "`<`",
            TokenKind::LtEq => "`<=`",
            TokenKind::Gt => "`>`",
            TokenKind::GtEq => "`>=`",
            TokenKind::Eof => "end of file",
        }
    }

    /// Maps an identifier to its keyword token, if it is one.
    ///
    /// The classic keyword table: the lexer reads one generic `Ident` and
    /// decides afterwards whether it was really a keyword.
    #[must_use]
    pub fn keyword(word: &str) -> Option<Self> {
        Some(match word {
            "let" => TokenKind::Let,
            "fn" => TokenKind::Fn,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            _ => return None,
        })
    }
}

impl fmt::Display for TokenKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Int(value) => write!(f, "{value}"),
            TokenKind::Float(value) => write!(f, "{value}"),
            TokenKind::Str(value) => write!(f, "{value:?}"),
            TokenKind::Ident(name) => write!(f, "{name}"),
            other => f.write_str(other.describe()),
        }
    }
}

/// A token: a kind plus where it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Token<'src> {
    pub kind: TokenKind<'src>,
    pub span: Span,
}

/// The zero-copy lexer.
///
/// It borrows the source for its whole lifetime, which is why the tokens it
/// produces can borrow from the same string.
pub struct Lexer<'src> {
    source: SourceId,
    src: &'src str,
    pos: usize,
    done: bool,
}

impl<'src> Lexer<'src> {
    /// Creates a lexer over `src`, tagging every span with `source`.
    #[must_use]
    pub const fn new(source: SourceId, src: &'src str) -> Self {
        Self {
            source,
            src,
            pos: 0,
            done: false,
        }
    }

    /// Byte offset the lexer has reached.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Whether the underlying input has been exhausted.
    #[must_use]
    pub const fn is_at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn rest(&self) -> &'src str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        let mut chars = self.rest().chars();
        chars.next();
        chars.next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn span_from(&self, start: usize) -> Span {
        Span::new(self.source, start as u32, self.pos as u32)
    }

    /// Skips whitespace and comments, returning at most one diagnostic.
    fn skip_trivia(&mut self) -> Option<Diagnostic> {
        loop {
            match self.peek() {
                Some(ch) if ch.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_second() == Some('/') => {
                    while let Some(ch) = self.peek() {
                        if ch == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('/') if self.peek_second() == Some('*') => {
                    let start = self.pos;
                    self.bump();
                    self.bump();
                    let mut depth = 1usize;
                    while depth > 0 {
                        match self.peek() {
                            None => {
                                return Some(
                                    Diagnostic::error("unterminated block comment")
                                        .with_code("E0002")
                                        .with_span(self.span_from(start)),
                                );
                            }
                            Some('/') if self.peek_second() == Some('*') => {
                                self.bump();
                                self.bump();
                                depth += 1;
                            }
                            Some('*') if self.peek_second() == Some('/') => {
                                self.bump();
                                self.bump();
                                depth -= 1;
                            }
                            Some(_) => {
                                self.bump();
                            }
                        }
                    }
                }
                _ => return None,
            }
        }
    }

    /// Lexes the next token, or `None` once the stream is finished.
    ///
    /// A lexer that has already emitted [`TokenKind::Eof`] returns `None`
    /// forever after, so the [`Iterator`] impl terminates.
    pub fn next_token(&mut self) -> Option<Result<Token<'src>, Diagnostic>> {
        if self.done {
            return None;
        }
        if let Some(diagnostic) = self.skip_trivia() {
            return Some(Err(diagnostic));
        }
        if self.is_at_end() {
            self.done = true;
            return Some(Ok(Token {
                kind: TokenKind::Eof,
                span: self.span_from(self.pos),
            }));
        }

        let start = self.pos;
        let ch = self.bump()?;
        let kind = match ch {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            '#' if self.peek() == Some('{') => {
                self.bump();
                TokenKind::HashBrace
            }
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '=' if self.peek() == Some('=') => {
                self.bump();
                TokenKind::Eq
            }
            '=' => TokenKind::Assign,
            '!' if self.peek() == Some('=') => {
                self.bump();
                TokenKind::NotEq
            }
            '!' => TokenKind::Not,
            '<' if self.peek() == Some('=') => {
                self.bump();
                TokenKind::LtEq
            }
            '<' => TokenKind::Lt,
            '>' if self.peek() == Some('=') => {
                self.bump();
                TokenKind::GtEq
            }
            '>' => TokenKind::Gt,
            '&' if self.peek() == Some('&') => {
                self.bump();
                TokenKind::And
            }
            '|' if self.peek() == Some('|') => {
                self.bump();
                TokenKind::Or
            }
            '"' => match self.lex_string(start) {
                Ok(kind) => kind,
                Err(diagnostic) => return Some(Err(diagnostic)),
            },
            '0'..='9' => self.lex_number(start),
            ch if ch == '_' || ch.is_alphabetic() => self.lex_identifier(start),
            other => {
                return Some(Err(Diagnostic::error(format!(
                    "unexpected character `{other}`"
                ))
                .with_code("E0001")
                .with_span(self.span_from(start))));
            }
        };

        Some(Ok(Token {
            kind,
            span: self.span_from(start),
        }))
    }

    fn lex_number(&mut self, start: usize) -> TokenKind<'src> {
        let mut is_float = false;
        while let Some(ch) = self.peek() {
            match ch {
                '0'..='9' | '_' => {
                    self.bump();
                }
                '.' if self.peek_second().is_some_and(|c| c.is_ascii_digit()) => {
                    is_float = true;
                    self.bump();
                }
                _ => break,
            }
        }
        let text = &self.src[start..self.pos];
        let cleaned: String = text.chars().filter(|&c| c != '_').collect();
        if is_float {
            TokenKind::Float(cleaned.parse().unwrap_or(f64::NAN))
        } else {
            match cleaned.parse::<i64>() {
                Ok(value) => TokenKind::Int(value),
                // Too big for i64: fall back to a float rather than failing.
                Err(_) => TokenKind::Float(cleaned.parse().unwrap_or(f64::NAN)),
            }
        }
    }

    fn lex_identifier(&mut self, start: usize) -> TokenKind<'src> {
        while let Some(ch) = self.peek() {
            if ch == '_' || ch.is_alphanumeric() {
                self.bump();
            } else {
                break;
            }
        }
        let word = &self.src[start..self.pos];
        TokenKind::keyword(word).unwrap_or(TokenKind::Ident(word))
    }

    fn lex_string(&mut self, start: usize) -> Result<TokenKind<'src>, Diagnostic> {
        let body_start = self.pos;
        let mut has_escape = false;
        loop {
            match self.peek() {
                None => {
                    return Err(Diagnostic::error("unterminated string literal")
                        .with_code("E0003")
                        .with_span(self.span_from(start)));
                }
                // Recovery point: Pebble strings are single-line, so a newline
                // ends the (broken) literal without swallowing the next line.
                Some('\n') => {
                    return Err(Diagnostic::error("unterminated string literal")
                        .with_code("E0003")
                        .with_span(self.span_from(start)));
                }
                Some('"') => break,
                Some('\\') => {
                    has_escape = true;
                    self.bump();
                    self.bump();
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
        let raw = &self.src[body_start..self.pos];
        self.bump(); // closing quote.

        if has_escape {
            Ok(TokenKind::Str(Cow::Owned(unescape(raw))))
        } else {
            Ok(TokenKind::Str(Cow::Borrowed(raw)))
        }
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Result<Token<'src>, Diagnostic>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

/// Lexes an entire source into tokens plus diagnostics, always ending in `Eof`.
///
/// Never fails, so callers can render every diagnostic before deciding what to
/// do with the (possibly partial) token stream.
#[must_use]
pub fn tokenize(source: SourceId, src: &str) -> (Vec<Token<'_>>, Vec<Diagnostic>) {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut lexer = Lexer::new(source, src);
    while let Some(item) = lexer.next_token() {
        match item {
            Ok(token) => {
                let is_eof = token.kind == TokenKind::Eof;
                tokens.push(token);
                if is_eof {
                    break;
                }
            }
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    (tokens, diagnostics)
}

fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('u') => out.push(read_unicode_escape(&mut chars)),
            // Unknown escape: keep it literally, backslash and all.
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => {}
        }
    }
    out
}

fn read_unicode_escape(chars: &mut core::str::Chars<'_>) -> char {
    // Expect `{XXXX}`; on anything malformed, emit U+FFFD.
    if chars.next() != Some('{') {
        return '\u{FFFD}';
    }
    let mut hex = String::new();
    for ch in chars.by_ref() {
        if ch == '}' {
            break;
        }
        hex.push(ch);
    }
    u32::from_str_radix(&hex, 16)
        .ok()
        .and_then(char::from_u32)
        .unwrap_or('\u{FFFD}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind<'_>> {
        let (tokens, diagnostics) = tokenize(SourceId(0), src);
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );
        tokens.into_iter().map(|token| token.kind).collect()
    }

    #[test]
    fn lexes_keywords_and_identifiers() {
        assert_eq!(
            kinds("let x = iffy;"),
            vec![
                TokenKind::Let,
                TokenKind::Ident("x"),
                TokenKind::Assign,
                TokenKind::Ident("iffy"),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn identifiers_borrow_the_source() {
        let source = String::from("hello");
        let (tokens, _) = tokenize(SourceId(0), &source);
        match &tokens[0].kind {
            TokenKind::Ident(name) => {
                let base = source.as_ptr() as usize;
                let borrowed = name.as_ptr() as usize;
                assert!((base..base + source.len()).contains(&borrowed));
            }
            other => panic!("expected identifier, got {other:?}"),
        }
    }

    #[test]
    fn string_without_escapes_is_borrowed() {
        let (tokens, _) = tokenize(SourceId(0), "\"plain\"");
        match &tokens[0].kind {
            TokenKind::Str(Cow::Borrowed(text)) => assert_eq!(*text, "plain"),
            other => panic!("expected borrowed string, got {other:?}"),
        }
    }

    #[test]
    fn string_with_escapes_is_owned_and_decoded() {
        assert_eq!(
            kinds(r#""a\nb\t\u{1F980}""#),
            vec![
                TokenKind::Str(Cow::Owned("a\nb\t🦀".into())),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn numbers_distinguish_int_and_float_with_separators() {
        assert_eq!(
            kinds("1_000 3.5"),
            vec![TokenKind::Int(1000), TokenKind::Float(3.5), TokenKind::Eof]
        );
    }

    #[test]
    fn multi_char_operators_win_over_single_char() {
        assert_eq!(
            kinds("== != <= >= && || = < > !"),
            vec![
                TokenKind::Eq,
                TokenKind::NotEq,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Assign,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::Not,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn keyword_and_symbol_spellings_agree() {
        assert_eq!(kinds("a and b or c not d"), kinds("a && b || c ! d"));
    }

    #[test]
    fn skips_line_and_nested_block_comments() {
        assert_eq!(
            kinds("1 // ignored\n/* a /* nested */ b */ 2"),
            vec![TokenKind::Int(1), TokenKind::Int(2), TokenKind::Eof]
        );
    }

    #[test]
    fn reports_unterminated_string_but_keeps_going() {
        let (tokens, diagnostics) = tokenize(SourceId(0), "\"oops\n42");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_deref(), Some("E0003"));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Int(42)));
    }

    #[test]
    fn reports_unexpected_character_with_a_span() {
        let (_, diagnostics) = tokenize(SourceId(0), "let x = @;");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_deref(), Some("E0001"));
        assert_eq!(diagnostics[0].labels[0].span.start, 8);
    }

    #[test]
    fn spans_cover_the_token_bytes() {
        let (tokens, _) = tokenize(SourceId(0), "let abc = 1;");
        assert_eq!(tokens[0].span, Span::new(SourceId(0), 0, 3)); // let
        assert_eq!(tokens[1].span, Span::new(SourceId(0), 4, 7)); // abc
    }

    #[test]
    fn iterator_terminates_after_eof() {
        let mut lexer = Lexer::new(SourceId(0), "1");
        let collected: Vec<_> = lexer.by_ref().collect();
        assert_eq!(collected.len(), 2); // Int(1), Eof
        assert_eq!(lexer.next(), None);
    }
}
