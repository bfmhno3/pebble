//! The command-line surface, declared with clap's derive API.
//!
//! Everything here is data: the actual work lives in [`crate::commands`] and
//! [`crate::repl`]. Keeping the shape of the interface in one small file makes
//! it easy to see at a glance what the driver can do.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Pebble: a small scripting language and toolchain.
#[derive(Debug, Parser)]
#[command(name = "pebble", version)]
pub struct Cli {
    /// The action to perform.
    #[command(subcommand)]
    pub command: Command,
}

/// Every top-level action the driver understands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Execute a source file on the tree-walker or the bytecode VM.
    Run(RunArgs),
    /// Start an interactive read-eval-print loop.
    Repl,
    /// Print the token stream of a source file.
    Tokens(FileArgs),
    /// Print the parsed abstract syntax tree of a source file.
    Parse(FileArgs),
    /// Lex, parse, and compile a source file without running it.
    Check(FileArgs),
    /// Time both backends on a source file.
    Bench(BenchArgs),
}

/// A subcommand whose only input is one source file.
#[derive(Debug, Args)]
pub struct FileArgs {
    /// The source file to read.
    pub file: PathBuf,
}

/// Arguments to `pebble run`.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// The source file to execute.
    pub file: PathBuf,
    /// Execute on the bytecode VM instead of the tree-walking interpreter.
    #[arg(long)]
    pub vm: bool,
    /// Print the parsed syntax tree before running.
    #[arg(long)]
    pub ast: bool,
    /// Print the compiled bytecode disassembly before running.
    #[arg(long)]
    pub dump_bytecode: bool,
    /// Do not print the program's final value.
    #[arg(long)]
    pub quiet: bool,
}

/// Arguments to `pebble bench`.
#[derive(Debug, Args)]
pub struct BenchArgs {
    /// The source file to time.
    pub file: PathBuf,
    /// How many times to run the file on each backend.
    #[arg(long, default_value_t = 5)]
    pub iters: u32,
}
