//! The work behind each subcommand.
//!
//! Every function returns the process exit code it wants: `0` on success and
//! `1` when the input produced diagnostics or failed at runtime. Only host
//! problems (an unreadable file, say) escape as an [`anyhow::Error`].

use std::time::{Duration, Instant};

use anyhow::Result;
use pebble_ast::{Program, pretty};
use pebble_bytecode::{Compiled, compile};
use pebble_interp::Interpreter;
use pebble_lexer::tokenize;
use pebble_parser::parse as parse_program;
use pebble_value::Buffer;
use pebble_vm::Vm;

use crate::cli::{BenchArgs, FileArgs, RunArgs};
use crate::sources::{self, Source};

/// `pebble run`: parse a file, then execute it on one of the two backends.
///
/// # Errors
/// Returns an error only for host-level failures such as an unreadable file.
pub fn run(args: &RunArgs) -> Result<u8> {
    let source = Source::load(&args.file)?;
    let parsed = parse_program(source.id, source.text());
    if sources::emit(&parsed.diagnostics, &source.map) {
        return Ok(1);
    }

    if args.ast {
        print!("{}", pretty(&parsed.program));
    }

    let mut compiled = None;
    if args.vm || args.dump_bytecode {
        let result = compile(&parsed.program);
        if sources::emit(&result.diagnostics, &source.map) {
            return Ok(1);
        }
        if args.dump_bytecode {
            print!("{}", result.compiled.chunk().disassemble());
        }
        compiled = Some(result.compiled);
    }

    if args.vm {
        let compiled = compiled.expect("`--vm` compiles the program first");
        Ok(run_vm(&compiled, &source, args.quiet))
    } else {
        Ok(run_interpreter(&parsed.program, &source, args.quiet))
    }
}

/// `pebble tokens`: one `line:col kind` line per token.
///
/// # Errors
/// Returns an error only for host-level failures such as an unreadable file.
pub fn tokens(args: &FileArgs) -> Result<u8> {
    let source = Source::load(&args.file)?;
    let (tokens, diagnostics) = tokenize(source.id, source.text());
    let failed = sources::emit(&diagnostics, &source.map);

    let file = source.map.file(source.id);
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
pub fn parse(args: &FileArgs) -> Result<u8> {
    let source = Source::load(&args.file)?;
    let parsed = parse_program(source.id, source.text());
    let failed = sources::emit(&parsed.diagnostics, &source.map);
    print!("{}", pretty(&parsed.program));
    Ok(if failed { 1 } else { 0 })
}

/// `pebble check`: lex, parse, and compile, reporting every diagnostic.
///
/// # Errors
/// Returns an error only for host-level failures such as an unreadable file.
pub fn check(args: &FileArgs) -> Result<u8> {
    let source = Source::load(&args.file)?;
    let parsed = parse_program(source.id, source.text());
    let mut failed = sources::emit(&parsed.diagnostics, &source.map);

    let compiled = compile(&parsed.program);
    failed |= sources::emit(&compiled.diagnostics, &source.map);

    Ok(if failed { 1 } else { 0 })
}

/// `pebble bench`: time both backends on the same file.
///
/// # Errors
/// Returns an error only for host-level failures such as an unreadable file.
pub fn bench(args: &BenchArgs) -> Result<u8> {
    let source = Source::load(&args.file)?;
    let parsed = parse_program(source.id, source.text());
    if sources::emit(&parsed.diagnostics, &source.map) {
        return Ok(1);
    }
    let compiled = compile(&parsed.program);
    if sources::emit(&compiled.diagnostics, &source.map) {
        return Ok(1);
    }

    let iterations = args.iters.max(1);

    // The backends print their own output; a buffer keeps the timings readable.
    let start = Instant::now();
    for _ in 0..iterations {
        let mut interpreter = Interpreter::with_output(Box::new(Buffer::new()));
        if let Err(error) = interpreter.interpret(&parsed.program) {
            sources::emit_runtime(&error, &source.map);
            return Ok(1);
        }
    }
    let tree_walker = start.elapsed();

    let start = Instant::now();
    for _ in 0..iterations {
        let mut vm = Vm::with_output(Box::new(Buffer::new()));
        if let Err(error) = vm.run(&compiled.compiled) {
            sources::emit_runtime(&error, &source.map);
            return Ok(1);
        }
    }
    let virtual_machine = start.elapsed();

    let count = f64::from(iterations);
    let interpreter_ms = millis(tree_walker);
    let vm_ms = millis(virtual_machine);
    println!(
        "tree-walker: {interpreter_ms:.3} ms over {iterations} iterations ({:.3} ms/iteration)",
        interpreter_ms / count
    );
    println!(
        "vm:          {vm_ms:.3} ms over {iterations} iterations ({:.3} ms/iteration)",
        vm_ms / count
    );
    Ok(0)
}

/// Executes with the tree-walking interpreter and prints the final value.
fn run_interpreter(program: &Program, source: &Source, quiet: bool) -> u8 {
    let mut interpreter = Interpreter::new();
    match interpreter.interpret(program) {
        Ok(value) => {
            // A script usually ends in a statement, so its value is `nil`;
            // printing that for every program would just be noise.
            if !quiet && !matches!(value, pebble_value::Value::Nil) {
                println!("{value}");
            }
            0
        }
        Err(error) => {
            sources::emit_runtime(&error, &source.map);
            1
        }
    }
}

/// Executes with the bytecode VM and prints the final value.
fn run_vm(compiled: &Compiled, source: &Source, quiet: bool) -> u8 {
    let mut vm = Vm::new();
    match vm.run(compiled) {
        Ok(value) => {
            if !quiet && value != pebble_vm::VmValue::Nil {
                println!("{}", vm.display(value));
            }
            0
        }
        Err(error) => {
            sources::emit_runtime(&error, &source.map);
            1
        }
    }
}

/// A duration in milliseconds, as a float.
fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
