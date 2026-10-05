#![forbid(unsafe_code)]
//! Shared programs and helpers for benchmarking Pebble's two backends.
//!
//! The benchmark harness (`benches/backends.rs`) and the correctness test at
//! the bottom of this file both consume the same three programs and the same
//! entry points, so the numbers and the equivalence check can never drift
//! apart.
//!
//! Both backends write to an in-memory [`Buffer`] rather than standard output,
//! so a program that calls `print` cannot pollute the benchmark transcript.

use pebble_interp::Interpreter;
use pebble_span::SourceMap;
use pebble_value::Buffer;
use pebble_vm::Vm;

/// A recursive `fib(20)`: the classic call/depth benchmark.
pub const FIB: &str = r#"
fn fib(n) {
    if n < 2 { return n; }
    return fib(n - 1) + fib(n - 2);
}
fib(20);
"#;

/// Sums `0..100_000` with a `while` loop: raw statement/arithmetic throughput.
pub const LOOP_SUM: &str = r#"
let i = 0;
let sum = 0;
while i < 100000 {
    sum = sum + i;
    i = i + 1;
}
sum;
"#;

/// Builds a 1000-element list with `push`, then reads it back by index.
pub const COLLECTIONS: &str = r#"
let xs = [];
let i = 0;
while i < 1000 {
    push(xs, i);
    i = i + 1;
}
let total = 0;
let j = 0;
while j < 1000 {
    total = total + xs[j];
    j = j + 1;
}
total;
"#;

/// Parses and compiles `src` to bytecode, panicking on any diagnostic.
///
/// # Panics
/// Panics with the rendered diagnostics if lexing, parsing, or compilation
/// reports an error. These programs are fixed constants, so a panic here means
/// the language or a backend changed incompatibly.
#[must_use]
pub fn compile_source(src: &str) -> pebble_bytecode::Compiled {
    let mut sources = SourceMap::new();
    let id = sources.add("<bench>", src);
    let parsed = pebble_parser::parse(id, src);
    if !parsed.diagnostics.is_empty() {
        panic_diagnostics(&sources, &parsed.diagnostics);
    }
    let result = pebble_bytecode::compile(&parsed.program);
    if !result.diagnostics.is_empty() {
        panic_diagnostics(&sources, &result.diagnostics);
    }
    result.compiled
}

/// Runs `src` on the tree-walking interpreter and returns the `Display` of the
/// program's final value.
///
/// # Panics
/// Panics on a parse or runtime error.
#[must_use]
pub fn run_interp(src: &str) -> String {
    let mut sources = SourceMap::new();
    let id = sources.add("<bench>", src);
    let parsed = pebble_parser::parse(id, src);
    if !parsed.diagnostics.is_empty() {
        panic_diagnostics(&sources, &parsed.diagnostics);
    }
    let mut interp = Interpreter::with_output(Box::new(Buffer::new()));
    let value = interp
        .interpret(&parsed.program)
        .unwrap_or_else(|error| panic!("interpreter error: {error}"));
    value.to_string()
}

/// Runs `src` on the bytecode VM and returns the rendered final value.
///
/// # Panics
/// Panics on a parse, compile, or runtime error.
#[must_use]
pub fn run_vm(src: &str) -> String {
    let compiled = compile_source(src);
    let mut vm = Vm::with_output(Box::new(Buffer::new()));
    let value = vm
        .run(&compiled)
        .unwrap_or_else(|error| panic!("vm error: {error}"));
    vm.display(value)
}

fn panic_diagnostics(sources: &SourceMap, diagnostics: &[pebble_diag::Diagnostic]) -> ! {
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.render(sources))
        .collect::<Vec<_>>()
        .join("\n");
    panic!("pebble diagnostics:\n{rendered}");
}

#[cfg(test)]
mod tests {
    use super::{COLLECTIONS, FIB, LOOP_SUM, run_interp, run_vm};

    #[test]
    fn both_backends_agree() {
        for (name, src) in [
            ("fib", FIB),
            ("loop_sum", LOOP_SUM),
            ("collections", COLLECTIONS),
        ] {
            assert_eq!(
                run_interp(src),
                run_vm(src),
                "backends disagree on `{name}`"
            );
        }
    }

    #[test]
    fn programs_produce_the_expected_values() {
        assert_eq!(run_interp(FIB), "6765");
        assert_eq!(run_vm(FIB), "6765");
        assert_eq!(run_interp(LOOP_SUM), "4999950000");
        assert_eq!(run_vm(LOOP_SUM), "4999950000");
        assert_eq!(run_interp(COLLECTIONS), "499500");
        assert_eq!(run_vm(COLLECTIONS), "499500");
    }
}
