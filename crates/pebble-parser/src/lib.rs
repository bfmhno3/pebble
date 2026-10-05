#![forbid(unsafe_code)]
//! The Pebble parser: tokens in, [`Program`] out.
//!
//! The parser is **recursive descent** for statements — one function per
//! construct — and **precedence climbing** (a small Pratt parser) for
//! expressions. Two things are worth noticing:
//!
//! * Errors are values. Every parsing function returns
//!   `Result<T, Diagnostic>`, and `?` threads failures upward. Nothing panics
//!   on bad input.
//! * It **recovers**. When a statement fails, [`Parser::synchronize`] skips to
//!   the next plausible statement boundary, so a single run reports many
//!   mistakes rather than stopping at the first.

use pebble_ast::{BinaryOp, Expr, Ident, LogicalOp, Program, Spanned as _, Stmt, UnaryOp};
use pebble_diag::Diagnostic;
use pebble_lexer::{Token, TokenKind, tokenize};
use pebble_span::{SourceId, Span};

/// The result of parsing a source file.
#[derive(Debug)]
pub struct ParseResult {
    /// Whatever AST could be recovered.
    pub program: Program,
    /// Lex and parse diagnostics, in source order as best we can manage.
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseResult {
    /// Whether parsing produced no diagnostics at all.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Parses a whole source file. Never fails; inspect [`ParseResult::diagnostics`].
#[must_use]
pub fn parse(source: SourceId, src: &str) -> ParseResult {
    let (tokens, mut diagnostics) = tokenize(source, src);
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program();
    diagnostics.append(&mut parser.errors);
    ParseResult {
        program,
        diagnostics,
    }
}

/// Parses a single expression, for the REPL. Returns `None` on error.
#[must_use]
pub fn parse_expression(source: SourceId, src: &str) -> (Option<Expr>, Vec<Diagnostic>) {
    let (tokens, mut diagnostics) = tokenize(source, src);
    let mut parser = Parser::new(tokens);
    match parser.expression() {
        Ok(expr) if parser.is_at_end() => {
            diagnostics.append(&mut parser.errors);
            (Some(expr), diagnostics)
        }
        Ok(_) => {
            let token = parser.peek().clone();
            diagnostics.push(
                Diagnostic::error(format!("unexpected {} after expression", token.kind))
                    .with_code("E1010")
                    .with_span(token.span),
            );
            diagnostics.append(&mut parser.errors);
            (None, diagnostics)
        }
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            diagnostics.append(&mut parser.errors);
            (None, diagnostics)
        }
    }
}

/// Internal marker for the operators handled by the expression ladder.
#[derive(Copy, Clone)]
enum Infix {
    Arithmetic(BinaryOp),
    Logic(LogicalOp),
}

/// The recursive-descent parser state.
pub struct Parser<'src> {
    tokens: Vec<Token<'src>>,
    current: usize,
    errors: Vec<Diagnostic>,
}

impl<'src> Parser<'src> {
    /// Wraps a token stream.
    #[must_use]
    pub fn new(tokens: Vec<Token<'src>>) -> Self {
        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
        }
    }

    /// Diagnostics collected while parsing.
    #[must_use]
    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    // --- token cursor -----------------------------------------------------

    fn peek(&self) -> &Token<'src> {
        // The lexer guarantees a trailing `Eof`, so indexing is always valid.
        &self.tokens[self.current.min(self.tokens.len() - 1)]
    }

    fn previous(&self) -> &Token<'src> {
        &self.tokens[self.current.saturating_sub(1)]
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn check(&self, kind: &TokenKind<'src>) -> bool {
        core::mem::discriminant(&self.peek().kind) == core::mem::discriminant(kind)
    }

    fn advance(&mut self) -> Token<'src> {
        let token = self.peek().clone();
        if !self.is_at_end() {
            self.current += 1;
        }
        token
    }

    fn matches(&mut self, kind: &TokenKind<'src>) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind<'src>, context: &str) -> Result<Token<'src>, Diagnostic> {
        if self.check(kind) {
            return Ok(self.advance());
        }
        let found = self.peek();
        Err(Diagnostic::error(format!(
            "expected {}, found {}",
            kind.describe(),
            found.kind
        ))
        .with_code("E1001")
        .with_span(found.span)
        .with_note(context.to_string()))
    }

    fn error<T>(&self, span: Span, message: impl Into<String>) -> Result<T, Diagnostic> {
        Err(Diagnostic::error(message)
            .with_code("E1001")
            .with_span(span))
    }

    // --- statements -------------------------------------------------------

    /// Parses statements until end of input, recovering from errors.
    pub fn parse_program(&mut self) -> Program {
        let mut stmts = Vec::new();
        while !self.is_at_end() {
            match self.declaration() {
                Ok(stmt) => stmts.push(stmt),
                Err(diagnostic) => {
                    self.errors.push(diagnostic);
                    self.synchronize();
                }
            }
        }
        Program::new(stmts)
    }

    fn declaration(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(&TokenKind::Fn) {
            self.fn_declaration()
        } else if self.check(&TokenKind::Let) {
            self.let_declaration()
        } else {
            self.statement()
        }
    }

    fn fn_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.advance().span; // `fn`
        let name_token = self.expect(&TokenKind::Ident(""), "a function needs a name")?;
        let name = self.ident_from(&name_token)?;
        let params = self.parameter_list()?;
        let body = self.block()?;
        let span = start.merge(self.previous().span);
        Ok(Stmt::Fn {
            name,
            params,
            body,
            span,
        })
    }

    fn let_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.advance().span; // `let`
        let name_token = self.expect(&TokenKind::Ident(""), "`let` needs a name")?;
        let name = self.ident_from(&name_token)?;
        self.expect(
            &TokenKind::Assign,
            "`let` needs an initialiser: `let x = ...;`",
        )?;
        let value = self.expression()?;
        let end = self
            .expect(&TokenKind::Semicolon, "statements end with `;`")?
            .span;
        Ok(Stmt::Let {
            name,
            value,
            span: start.merge(end),
        })
    }

    fn statement(&mut self) -> Result<Stmt, Diagnostic> {
        if self.matches(&TokenKind::If) {
            self.if_statement(self.previous().span)
        } else if self.matches(&TokenKind::While) {
            self.while_statement(self.previous().span)
        } else if self.matches(&TokenKind::For) {
            self.for_statement(self.previous().span)
        } else if self.matches(&TokenKind::Return) {
            self.return_statement(self.previous().span)
        } else if self.matches(&TokenKind::Break) {
            let start = self.previous().span;
            let end = self.expect(&TokenKind::Semicolon, "`break;`")?.span;
            Ok(Stmt::Break {
                span: start.merge(end),
            })
        } else if self.matches(&TokenKind::Continue) {
            let start = self.previous().span;
            let end = self.expect(&TokenKind::Semicolon, "`continue;`")?.span;
            Ok(Stmt::Continue {
                span: start.merge(end),
            })
        } else {
            self.expression_statement()
        }
    }

    fn if_statement(&mut self, start: Span) -> Result<Stmt, Diagnostic> {
        let cond = self.expression()?;
        let then_branch = self.block()?;
        let mut else_branch = Vec::new();
        let end;
        if self.matches(&TokenKind::Else) {
            if self.matches(&TokenKind::If) {
                // `else if` desugars into a one-statement else block.
                let nested = self.if_statement(self.previous().span)?;
                end = nested.span();
                else_branch.push(nested);
            } else {
                let block = self.block()?;
                end = self.previous().span;
                else_branch = block;
            }
        } else {
            end = self.previous().span;
        }
        Ok(Stmt::If {
            cond,
            then_branch,
            else_branch,
            span: start.merge(end),
        })
    }

    fn while_statement(&mut self, start: Span) -> Result<Stmt, Diagnostic> {
        let cond = self.expression()?;
        let body = self.block()?;
        let span = start.merge(self.previous().span);
        Ok(Stmt::While { cond, body, span })
    }

    fn for_statement(&mut self, start: Span) -> Result<Stmt, Diagnostic> {
        let var_token = self.expect(&TokenKind::Ident(""), "`for` needs a loop variable")?;
        let var = self.ident_from(&var_token)?;
        self.expect(&TokenKind::In, "`for x in iterable { .. }`")?;
        let iter = self.expression()?;
        let body = self.block()?;
        let span = start.merge(self.previous().span);
        Ok(Stmt::For {
            var,
            iter,
            body,
            span,
        })
    }

    fn return_statement(&mut self, start: Span) -> Result<Stmt, Diagnostic> {
        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        let end = self
            .expect(&TokenKind::Semicolon, "statements end with `;`")?
            .span;
        Ok(Stmt::Return {
            value,
            span: start.merge(end),
        })
    }

    fn expression_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let expr = self.expression()?;
        let span = expr.span();
        let end = self
            .expect(
                &TokenKind::Semicolon,
                "expression statements are terminated with `;`",
            )?
            .span;
        Ok(Stmt::Expr {
            expr,
            span: span.merge(end),
        })
    }

    fn block(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.expect(&TokenKind::LBrace, "expected a `{`-delimited block")?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            match self.declaration() {
                Ok(stmt) => stmts.push(stmt),
                Err(diagnostic) => {
                    self.errors.push(diagnostic);
                    self.synchronize();
                }
            }
        }
        self.expect(&TokenKind::RBrace, "this `{` is never closed")?;
        Ok(stmts)
    }

    fn parameter_list(&mut self) -> Result<Vec<Ident>, Diagnostic> {
        self.expect(&TokenKind::LParen, "parameters are wrapped in `(` `)`")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let token = self.expect(&TokenKind::Ident(""), "expected a parameter name")?;
                params.push(self.ident_from(&token)?);
                if !self.matches(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen, "unclosed parameter list")?;
        Ok(params)
    }

    // --- expressions ------------------------------------------------------

    /// Parses a full expression, including assignment.
    pub fn expression(&mut self) -> Result<Expr, Diagnostic> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, Diagnostic> {
        let expr = self.binary(0)?;
        if self.matches(&TokenKind::Assign) {
            let value = self.assignment()?; // right-associative
            if !is_assignable(&expr) {
                return self.error(expr.span(), "invalid assignment target");
            }
            let span = expr.span().merge(value.span());
            return Ok(Expr::Assign {
                target: Box::new(expr),
                value: Box::new(value),
                span,
            });
        }
        Ok(expr)
    }

    /// Precedence climbing over the infix operators.
    ///
    /// `min_prec` is the level we are allowed to consume; higher binds tighter.
    fn binary(&mut self, min_prec: u8) -> Result<Expr, Diagnostic> {
        let mut left = self.unary()?;
        while let Some((prec, infix)) = infix_op(&self.peek().kind) {
            if prec < min_prec {
                break;
            }
            self.advance();
            // Left-associative: recurse one level higher.
            let right = self.binary(prec + 1)?;
            let span = left.span().merge(right.span());
            left = match infix {
                Infix::Arithmetic(op) => Expr::Binary {
                    op,
                    lhs: Box::new(left),
                    rhs: Box::new(right),
                    span,
                },
                Infix::Logic(op) => Expr::Logical {
                    op,
                    lhs: Box::new(left),
                    rhs: Box::new(right),
                    span,
                },
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.matches(&TokenKind::Minus) {
            let start = self.previous().span;
            let operand = self.unary()?;
            let span = start.merge(operand.span());
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                span,
            });
        }
        if self.matches(&TokenKind::Not) {
            let start = self.previous().span;
            let operand = self.unary()?;
            let span = start.merge(operand.span());
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                span,
            });
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.primary()?;
        loop {
            if self.matches(&TokenKind::LParen) {
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.expression()?);
                        if !self.matches(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                let end = self
                    .expect(&TokenKind::RParen, "unclosed argument list")?
                    .span;
                let span = expr.span().merge(end);
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else if self.matches(&TokenKind::LBracket) {
                let index = self.expression()?;
                let end = self.expect(&TokenKind::RBracket, "unclosed index")?.span;
                let span = expr.span().merge(end);
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.advance();
        let span = token.span;
        match token.kind {
            TokenKind::Nil => Ok(Expr::Nil { span }),
            TokenKind::True => Ok(Expr::Bool { value: true, span }),
            TokenKind::False => Ok(Expr::Bool { value: false, span }),
            TokenKind::Int(value) => Ok(Expr::Int { value, span }),
            TokenKind::Float(value) => Ok(Expr::Float { value, span }),
            TokenKind::Str(value) => Ok(Expr::Str {
                value: value.into_owned(),
                span,
            }),
            TokenKind::Ident(name) => Ok(Expr::Var {
                name: Ident::new(name, span),
                span,
            }),
            TokenKind::LParen => {
                let expr = self.expression()?;
                self.expect(&TokenKind::RParen, "unclosed `(`")?;
                Ok(expr)
            }
            TokenKind::LBracket => self.list_literal(span),
            TokenKind::HashBrace => self.map_literal(span),
            TokenKind::Fn => {
                let params = self.parameter_list()?;
                let body = self.block()?;
                let span = span.merge(self.previous().span);
                Ok(Expr::Fn { params, body, span })
            }
            other => self.error(span, format!("expected an expression, found {other}")),
        }
    }

    fn list_literal(&mut self, start: Span) -> Result<Expr, Diagnostic> {
        let mut items = Vec::new();
        if !self.check(&TokenKind::RBracket) {
            loop {
                items.push(self.expression()?);
                if !self.matches(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RBracket) {
                    break; // trailing comma
                }
            }
        }
        let end = self.expect(&TokenKind::RBracket, "unclosed `[`")?.span;
        Ok(Expr::List {
            items,
            span: start.merge(end),
        })
    }

    fn map_literal(&mut self, start: Span) -> Result<Expr, Diagnostic> {
        let mut entries = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                let key = self.expression()?;
                self.expect(&TokenKind::Colon, "map entries look like `key: value`")?;
                let value = self.expression()?;
                entries.push((key, value));
                if !self.matches(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RBrace) {
                    break; // trailing comma
                }
            }
        }
        let end = self.expect(&TokenKind::RBrace, "unclosed `#{`")?.span;
        Ok(Expr::Map {
            entries,
            span: start.merge(end),
        })
    }

    // --- helpers ----------------------------------------------------------

    fn ident_from(&self, token: &Token<'src>) -> Result<Ident, Diagnostic> {
        match &token.kind {
            TokenKind::Ident(name) => Ok(Ident::new(*name, token.span)),
            other => self.error(token.span, format!("expected an identifier, found {other}")),
        }
    }

    /// Skips tokens until we are probably at the start of the next statement.
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            if self.previous().kind == TokenKind::Semicolon {
                return;
            }
            match self.peek().kind {
                TokenKind::Fn
                | TokenKind::Let
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::RBrace => return,
                _ => {
                    self.advance();
                }
            }
        }
    }
}

fn is_assignable(expr: &Expr) -> bool {
    matches!(expr, Expr::Var { .. } | Expr::Index { .. })
}

/// Infix operators mapped to `(precedence, meaning)`. Higher binds tighter.
fn infix_op(kind: &TokenKind<'_>) -> Option<(u8, Infix)> {
    Some(match kind {
        TokenKind::Or => (1, Infix::Logic(LogicalOp::Or)),
        TokenKind::And => (2, Infix::Logic(LogicalOp::And)),
        TokenKind::Eq => (3, Infix::Arithmetic(BinaryOp::Eq)),
        TokenKind::NotEq => (3, Infix::Arithmetic(BinaryOp::NotEq)),
        TokenKind::Lt => (4, Infix::Arithmetic(BinaryOp::Lt)),
        TokenKind::LtEq => (4, Infix::Arithmetic(BinaryOp::LtEq)),
        TokenKind::Gt => (4, Infix::Arithmetic(BinaryOp::Gt)),
        TokenKind::GtEq => (4, Infix::Arithmetic(BinaryOp::GtEq)),
        TokenKind::Plus => (5, Infix::Arithmetic(BinaryOp::Add)),
        TokenKind::Minus => (5, Infix::Arithmetic(BinaryOp::Sub)),
        TokenKind::Star => (6, Infix::Arithmetic(BinaryOp::Mul)),
        TokenKind::Slash => (6, Infix::Arithmetic(BinaryOp::Div)),
        TokenKind::Percent => (6, Infix::Arithmetic(BinaryOp::Rem)),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pebble_ast::pretty;

    fn parse_ok(src: &str) -> Program {
        let result = parse(SourceId(0), src);
        assert!(
            result.is_ok(),
            "expected clean parse, got {:?}",
            result.diagnostics
        );
        result.program
    }

    #[test]
    fn parses_precedence_correctly() {
        // 1 + 2 * 3 must group as 1 + (2 * 3).
        let program = parse_ok("1 + 2 * 3;");
        let rendered = pretty(&program);
        assert_eq!(
            rendered,
            "expr\n  binary +\n    int 1\n    binary *\n      int 2\n      int 3\n"
        );
    }

    #[test]
    fn parentheses_override_precedence() {
        let program = parse_ok("(1 + 2) * 3;");
        let rendered = pretty(&program);
        assert!(
            rendered.starts_with("expr\n  binary *\n    binary +\n"),
            "{rendered}"
        );
    }

    #[test]
    fn subtraction_is_left_associative() {
        // 1 - 2 - 3 == (1 - 2) - 3
        let program = parse_ok("1 - 2 - 3;");
        let rendered = pretty(&program);
        assert_eq!(
            rendered,
            "expr\n  binary -\n    binary -\n      int 1\n      int 2\n    int 3\n"
        );
    }

    #[test]
    fn parses_let_and_variables() {
        let program = parse_ok("let x = 1; x = x + 2;");
        assert_eq!(program.stmts.len(), 2);
        assert!(matches!(program.stmts[1], Stmt::Expr { .. }));
    }

    #[test]
    fn rejects_assignment_to_literal() {
        let result = parse(SourceId(0), "1 = 2;");
        assert!(!result.is_ok());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("invalid assignment"))
        );
    }

    #[test]
    fn parses_functions_and_returns() {
        let program = parse_ok("fn add(a, b) { return a + b; }");
        let rendered = pretty(&program);
        assert!(rendered.contains("fn add(a, b)"), "{rendered}");
        assert!(rendered.contains("return"), "{rendered}");
    }

    #[test]
    fn parses_closures_lists_and_maps() {
        let program =
            parse_ok("let f = fn(x) { return x; }; let xs = [1, 2,]; let m = #{ \"a\": 1, };");
        let rendered = pretty(&program);
        assert!(rendered.contains("fn(x)"), "{rendered}");
        assert!(rendered.contains("list[2]"), "{rendered}");
        assert!(rendered.contains("map{1 entries}"), "{rendered}");
    }

    #[test]
    fn parses_control_flow() {
        let program = parse_ok(
            "if a { b(); } else if c { d(); } else { e(); } while a { b(); } for i in xs { c(i); }",
        );
        assert_eq!(program.stmts.len(), 3);
    }

    #[test]
    fn parses_break_and_continue() {
        parse_ok("while a { if b { break; } continue; }");
    }

    #[test]
    fn recovers_and_reports_multiple_errors() {
        let result = parse(SourceId(0), "let = ;\nlet y = 2\nlet z = 3;");
        assert!(result.diagnostics.len() >= 2, "{:?}", result.diagnostics);
        // Recovery still yields the later, valid statement.
        assert!(result.program.stmts.iter().any(|s| matches!(
            s,
            Stmt::Let { name, .. } if name.name == "z"
        )));
    }

    #[test]
    fn reports_missing_semicolon_with_a_span() {
        let result = parse(SourceId(0), "let x = 1");
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code.as_deref(), Some("E1001"));
    }

    #[test]
    fn parse_expression_entry_point_works() {
        let (expr, diagnostics) = parse_expression(SourceId(0), "1 + 2");
        assert!(diagnostics.is_empty());
        assert!(expr.is_some());

        let (expr, diagnostics) = parse_expression(SourceId(0), "1 + ");
        assert!(expr.is_none());
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn skips_empty_statements_never_panics_on_garbage() {
        // Fuzz-ish: throw a pile of punctuation at it and require no panic.
        for junk in ["", ";;;", ")))", "{{{", "let", "fn", "a[", "#{", "1 + + 2;"] {
            let result = parse(SourceId(0), junk);
            let _ = result.program;
        }
    }
}
