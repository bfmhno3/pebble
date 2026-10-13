//! The `pebble` command-line driver.
//!
//! This binary is the front door to the toolchain: it wires the lexer, the
//! parser, both execution backends, and the diagnostics renderer together
//! behind a handful of subcommands.
//!
//! Exit codes follow the usual convention: `0` for success, `1` when the input
//! produced diagnostics or failed at runtime, and `2` (clap's default) for a
//! usage error.

#![forbid(unsafe_code)]

mod backend;
mod cli;
mod commands;
#[cfg(feature = "repl")]
mod repl;
mod sources;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Run(args) => commands::run(&args),
        #[cfg(feature = "repl")]
        Command::Repl => repl::run(),
        Command::Tokens(args) => commands::tokens(&args),
        Command::Parse(args) => commands::parse(&args),
        Command::Check(args) => commands::check(&args),
        Command::Bench(args) => commands::bench(&args),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("pebble: {error:#}");
            ExitCode::from(1)
        }
    }
}
