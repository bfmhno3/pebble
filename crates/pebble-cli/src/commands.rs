//! The work behind each subcommand.
//!
//! Every function returns the process exit code it wants: `0` on success and
//! `1` when the input produced diagnostics or failed at runtime. Only host
//! problems (an unreadable file, say) escape as an [`anyhow::Error`].

use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use pebble_ast::{Program, pretty};
use pebble_bytecode::compile;
use pebble_lexer::tokenize;
use pebble_parser::parse as parse_program;
use pebble_value::{Buffer, Stdout};

use crate::backend::{Backend, BytecodeVm, TreeWalker};
use crate::cli::{BenchArgs, CheckArgs, FileArgs, RunArgs};
use crate::sources::{self, Source};

/// `pebble run`: parse a file, then execute it on one of the two backends.
///
/// # Errors
/// Returns an error only for host-level failures such as an unreadable file.
pub(crate) fn run(args: &RunArgs) -> Result<u8> {
    let source = Source::load(&args.file)?;
    let parsed = parse_program(source.id, source.text());
    if sources::emit(&parsed.diagnostics, &source.map) {
        return Ok(1);
    }

    if args.ast {
        print!("{}", pretty(&parsed.program));
    }

    if args.vm {
        let Some(prepared) = prepare_or_report::<BytecodeVm>(&parsed.program, &source) else {
            return Ok(1);
        };
        if args.dump_bytecode {
            print!("{}", prepared.chunk().disassemble());
        }
        return Ok(execute::<BytecodeVm>(&prepared, &source, args.quiet));
    }

    if args.dump_bytecode {
        let Some(prepared) = prepare_or_report::<BytecodeVm>(&parsed.program, &source) else {
            return Ok(1);
        };
        print!("{}", prepared.chunk().disassemble());
    }
    let Some(prepared) = prepare_or_report::<TreeWalker>(&parsed.program, &source) else {
        return Ok(1);
    };
    Ok(execute::<TreeWalker>(&prepared, &source, args.quiet))
}

/// `pebble tokens`: one `line:col kind` line per token.
///
/// # Errors
/// Returns an error only for host-level failures such as an unreadable file.
pub(crate) fn tokens(args: &FileArgs) -> Result<u8> {
    let source = Source::load(&args.file)?;
    let (tokens, diagnostics) = tokenize(source.id, source.text());
    let failed = sources::emit(&diagnostics, &source.map);

    let file = &source.map[source.id];
    for token in &tokens {
        let position = file.position(token.span.start);
        println!("{}:{} {}", position.line, position.column, token.kind);
    }
    Ok(if failed { 1 } else { 0 })
}

/// `pebble parse`: print the recovered syntax tree.
///
/// # Errors
/// Returns an error only for host-level failures such as an unreadable file.
pub(crate) fn parse(args: &FileArgs) -> Result<u8> {
    let source = Source::load(&args.file)?;
    let parsed = parse_program(source.id, source.text());
    let failed = sources::emit(&parsed.diagnostics, &source.map);
    print!("{}", pretty(&parsed.program));
    Ok(if failed { 1 } else { 0 })
}

/// The outcome of checking one file: its rendered diagnostics and whether any
/// of them is an error.
struct CheckReport {
    rendered: String,
    failed: bool,
}

/// `pebble check`: lex, parse, and compile every file, each on its own thread,
/// then print the reports in command-line order.
///
/// # Errors
/// Returns the first host-level failure (such as an unreadable file), in
/// command-line order.
pub(crate) fn check(args: &CheckArgs) -> Result<u8> {
    let reports: Vec<Result<CheckReport>> = thread::scope(|scope| {
        let handles: Vec<_> = args
            .files
            .iter()
            .map(|path| scope.spawn(|| check_file(path)))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a check thread panicked"))
            .collect()
    });

    let mut failed = false;
    for report in reports {
        let report = report?;
        eprint!("{}", report.rendered);
        failed |= report.failed;
    }
    Ok(u8::from(failed))
}

/// Checks one file. Everything non-`Send` (the compiled chunk holds `Rc`s)
/// stays inside this function; only the rendered text leaves the thread.
fn check_file(path: &Path) -> Result<CheckReport> {
    let source = Source::load(path)?;
    let parsed = parse_program(source.id, source.text());
    let compiled = compile(&parsed.program);

    let mut rendered = sources::render(&parsed.diagnostics, &source.map);
    rendered.push_str(&sources::render(&compiled.diagnostics, &source.map));
    let failed =
        sources::has_errors(&parsed.diagnostics) || sources::has_errors(&compiled.diagnostics);
    Ok(CheckReport { rendered, failed })
}

/// `pebble bench`: time both backends on the same file.
///
/// # Errors
/// Returns an error only for host-level failures such as an unreadable file.
pub(crate) fn bench(args: &BenchArgs) -> Result<u8> {
    let source = Source::load(&args.file)?;
    let parsed = parse_program(source.id, source.text());
    if sources::emit(&parsed.diagnostics, &source.map) {
        return Ok(1);
    }
    let Some(tree) = prepare_or_report::<TreeWalker>(&parsed.program, &source) else {
        return Ok(1);
    };
    let Some(bytecode) = prepare_or_report::<BytecodeVm>(&parsed.program, &source) else {
        return Ok(1);
    };

    let iterations = args.iters.max(1);
    let Some(tree_walker) = time::<TreeWalker>(&tree, iterations, &source) else {
        return Ok(1);
    };
    let Some(virtual_machine) = time::<BytecodeVm>(&bytecode, iterations, &source) else {
        return Ok(1);
    };

    report::<TreeWalker>(tree_walker, iterations);
    report::<BytecodeVm>(virtual_machine, iterations);
    Ok(0)
}

/// Prepares `program` for backend `B`, printing diagnostics on failure.
fn prepare_or_report<'p, B: Backend>(
    program: &'p Program,
    source: &Source,
) -> Option<B::Prepared<'p>> {
    match B::prepare(program) {
        Ok(prepared) => Some(prepared),
        Err(diagnostics) => {
            sources::emit(&diagnostics, &source.map);
            None
        }
    }
}

/// Runs a prepared program on a fresh `B` that prints to stdout, and returns
/// the exit code.
fn execute<B: Backend>(prepared: &B::Prepared<'_>, source: &Source, quiet: bool) -> u8 {
    let mut backend = B::with_output(Box::new(Stdout));
    match backend.execute(prepared) {
        Ok(value) => {
            if let Some(text) = value
                && !quiet
            {
                println!("{text}");
            }
            0
        }
        Err(error) => {
            sources::emit_runtime(&error, &source.map);
            1
        }
    }
}

/// Runs a prepared program `iterations` times on fresh engines and returns the
/// total time, or `None` after reporting a runtime error.
///
/// The backends print their own output; a buffer keeps the timings readable.
fn time<B: Backend>(
    prepared: &B::Prepared<'_>,
    iterations: u32,
    source: &Source,
) -> Option<Duration> {
    let start = Instant::now();
    for _ in 0..iterations {
        let mut backend = B::with_output(Box::new(Buffer::new()));
        if let Err(error) = backend.execute(prepared) {
            sources::emit_runtime(&error, &source.map);
            return None;
        }
    }
    Some(start.elapsed())
}

/// Prints one `pebble bench` result line.
fn report<B: Backend>(total: Duration, iterations: u32) {
    let total = millis(total);
    let per_iteration = total / f64::from(iterations);
    println!(
        "{:<12} {total:.3} ms over {iterations} iterations ({per_iteration:.3} ms/iteration)",
        format!("{}:", B::NAME)
    );
}

/// A duration in milliseconds, as a float.
fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
