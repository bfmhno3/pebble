//! The interactive read-eval-print loop.
//!
//! The interpreter is created once, so definitions persist from line to line,
//! but every line gets its own [`pebble_span::SourceMap`]: a line is parsed and
//! rendered in isolation, which keeps offsets small and diagnostics exact.

use std::str::FromStr;

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
                    match trimmed.parse::<MetaCommand>() {
                        Ok(MetaCommand::Quit) => break,
                        Ok(MetaCommand::Help) => print_help(),
                        Ok(MetaCommand::Tokens(code)) => show_tokens(&code),
                        Ok(MetaCommand::Ast(code)) => show_ast(&code),
                        Err(message) => eprintln!("pebble: {message}"),
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

/// A `:`-prefixed REPL command.
#[derive(Debug, PartialEq, Eq)]
enum MetaCommand {
    /// Leave the REPL.
    Quit,
    /// Show the command list.
    Help,
    /// Print the token stream of the argument.
    Tokens(String),
    /// Print the syntax tree of the argument.
    Ast(String),
}

impl FromStr for MetaCommand {
    type Err = String;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let Some(rest) = line.strip_prefix(':') else {
            return Err(format!("`{line}` is not a command"));
        };
        let (name, argument) = match rest.split_once(char::is_whitespace) {
            Some((name, argument)) => (name, argument.trim()),
            None => (rest, ""),
        };
        match name {
            "quit" | "q" => Ok(Self::Quit),
            "help" | "h" => Ok(Self::Help),
            "tokens" => Ok(Self::Tokens(argument.to_owned())),
            "ast" => Ok(Self::Ast(argument.to_owned())),
            other => Err(format!("unknown command `:{other}`; try :help")),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_has_a_short_form() {
        assert_eq!(":q".parse(), Ok(MetaCommand::Quit));
    }

    #[test]
    fn arguments_are_kept_whole() {
        assert_eq!(
            ":tokens 1 + 2".parse(),
            Ok(MetaCommand::Tokens("1 + 2".to_owned()))
        );
    }

    #[test]
    fn unknown_commands_are_rejected() {
        assert_eq!(
            ":nope".parse::<MetaCommand>(),
            Err("unknown command `:nope`; try :help".to_owned())
        );
    }
}
