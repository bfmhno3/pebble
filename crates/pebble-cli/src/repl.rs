//! The interactive read-eval-print loop.
//!
//! The interpreter is created once, so definitions persist from line to line,
//! but every line gets its own [`pebble_span::SourceMap`]: a line is parsed and
//! rendered in isolation, which keeps offsets small and diagnostics exact.

use anyhow::{Result, anyhow};
use pebble_ast::{Program, Spanned as _, Stmt, pretty};
use pebble_interp::Interpreter;
use pebble_lexer::tokenize;
use pebble_parser::{parse, parse_expression};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::sources::{self, Source};

/// The text shown before every input line.
const PROMPT: &str = "pebble> ";

/// Runs the REPL until the user quits or sends end-of-file.
///
/// # Errors
/// Returns an error if the line editor cannot be initialised.
pub(crate) fn run() -> Result<u8> {
    let mut editor =
        DefaultEditor::new().map_err(|error| anyhow!("cannot start the line editor: {error}"))?;
    let mut interpreter = Interpreter::new();

    println!("Pebble REPL. Type :help for commands, :quit to exit.");
    loop {
        match editor.readline(PROMPT) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let _ = editor.add_history_entry(line.as_str());
                if trimmed.starts_with(':') {
                    if meta_command(trimmed) {
                        break;
                    }
                } else {
                    eval_line(trimmed, &mut interpreter);
                }
            }
            // Ctrl-C and Ctrl-D both leave the session cleanly.
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(error) => {
                eprintln!("pebble: input error: {error}");
                break;
            }
        }
    }
    println!();
    Ok(0)
}

/// Handles a `:`-prefixed command. Returns `true` to leave the REPL.
fn meta_command(line: &str) -> bool {
    let Some(rest) = line.strip_prefix(':') else {
        return false;
    };
    let (name, argument) = match rest.split_once(char::is_whitespace) {
        Some((name, argument)) => (name, argument.trim()),
        None => (rest, ""),
    };
    match name {
        "quit" | "q" => return true,
        "help" | "h" => print_help(),
        "tokens" => show_tokens(argument),
        "ast" => show_ast(argument),
        other => eprintln!("pebble: unknown command `:{other}`; try :help"),
    }
    false
}

/// Prints the list of meta commands.
fn print_help() {
    println!("commands:");
    println!("  :help, :h        show this help");
    println!("  :quit, :q        leave the REPL");
    println!("  :tokens <code>   print the token stream for <code>");
    println!("  :ast <code>      print the parsed syntax tree for <code>");
}

/// `:tokens` — lex a snippet and print one `line:col kind` line per token.
fn show_tokens(code: &str) {
    if code.is_empty() {
        eprintln!("usage: :tokens <code>");
        return;
    }
    let source = Source::from_text("<repl>", code);
    let (tokens, diagnostics) = tokenize(source.id, source.text());
    sources::emit(&diagnostics, &source.map);

    let file = &source.map[source.id];
    for token in &tokens {
        let position = file.position(token.span.start);
        println!("{}:{} {}", position.line, position.column, token.kind);
    }
}

/// `:ast` — parse a snippet and print its syntax tree.
fn show_ast(code: &str) {
    if code.is_empty() {
        eprintln!("usage: :ast <code>");
        return;
    }
    let source = Source::from_text("<repl>", code);
    let parsed = parse(source.id, source.text());
    sources::emit(&parsed.diagnostics, &source.map);
    print!("{}", pretty(&parsed.program));
}

/// Evaluates one input line: an expression if it parses as one, a program
/// otherwise.
fn eval_line(line: &str, interpreter: &mut Interpreter) {
    let source = Source::from_text("<repl>", line);

    let (expression, diagnostics) = parse_expression(source.id, source.text());
    if let Some(expression) = expression {
        sources::emit(&diagnostics, &source.map);
        let span = expression.span();
        let program = Program::new(vec![Stmt::Expr {
            expr: expression,
            span,
        }]);
        match interpreter.interpret(&program) {
            Ok(value) => println!("{value}"),
            Err(error) => sources::emit_runtime(&error, &source.map),
        }
        return;
    }

    // Not an expression: treat the line as a small program and execute it.
    let parsed = parse(source.id, source.text());
    if sources::emit(&parsed.diagnostics, &source.map) {
        return;
    }
    if let Err(error) = interpreter.interpret(&parsed.program) {
        sources::emit_runtime(&error, &source.map);
    }
}
