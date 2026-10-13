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
pub(crate) struct Cli {
    /// The action to perform.
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// Every top-level action the driver understands.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Execute a source file on the tree-walker or the bytecode VM.
    Run(RunArgs),
    /// Start an interactive read-eval-print loop.
    Repl,
    /// Print the token stream of a source file.
    Tokens(FileArgs),
    /// Print the parsed abstract syntax tree of a source file.
    Parse(FileArgs),
    /// Lex, parse, and compile source files without running them.
    Check(CheckArgs),
    /// Time both backends on a source file.
    Bench(BenchArgs),
}

/// A subcommand whose only input is one source file.
#[derive(Debug, Args)]
pub(crate) struct FileArgs {
    /// The source file to read.
    pub(crate) file: PathBuf,
}

/// Arguments to `pebble check`.
#[derive(Debug, Args)]
pub(crate) struct CheckArgs {
    /// The source files to check; each one is checked on its own thread.
    #[arg(required = true)]
    pub(crate) files: Vec<PathBuf>,
}

/// Arguments to `pebble run`.
#[derive(Debug, Args)]
pub(crate) struct RunArgs {
    /// The source file to execute.
    pub(crate) file: PathBuf,
    /// Execute on the bytecode VM instead of the tree-walking interpreter.
    #[arg(long)]
    pub(crate) vm: bool,
    /// Print the parsed syntax tree before running.
    #[arg(long)]
    pub(crate) ast: bool,
    /// Print the compiled bytecode disassembly before running.
    #[arg(long)]
    pub(crate) dump_bytecode: bool,
    /// Do not print the program's final value.
    #[arg(long)]
    pub(crate) quiet: bool,
}

/// Arguments to `pebble bench`.
#[derive(Debug, Args)]
pub(crate) struct BenchArgs {
    /// The source file to time.
    pub(crate) file: PathBuf,
    /// How many times to run the file on each backend.
    #[arg(long, default_value_t = 5)]
    pub(crate) iters: u32,
}
