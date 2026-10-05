//! End-to-end tests for the `pebble` binary.
//!
//! The tests drive the real executable against the example programs in the
//! workspace's `examples/` directory, which is why every command runs with the
//! workspace root as its working directory.

use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use predicates::prelude::*;

/// The workspace root, two levels above this crate.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A `pebble` invocation rooted at the workspace, so `examples/...` resolves.
fn pebble() -> Command {
    let mut command = Command::cargo_bin("pebble").expect("the pebble binary is built");
    command.current_dir(workspace_root());
    command
}

/// Runs the binary with `args` and captures its output.
fn pebble_output(args: &[&str]) -> Output {
    pebble().args(args).output().expect("pebble runs")
}

#[test]
fn run_evaluates_fibonacci() {
    pebble()
        .args(["run", "examples/fib.pebble"])
        .assert()
        .success()
        .stdout(predicate::str::contains("6765"));
}

#[test]
fn the_vm_and_the_interpreter_agree() {
    let interpreted = pebble_output(&["run", "examples/fib.pebble"]);
    let compiled = pebble_output(&["run", "--vm", "examples/fib.pebble"]);

    assert!(
        interpreted.status.success(),
        "interpreter failed: {interpreted:?}"
    );
    assert!(compiled.status.success(), "vm failed: {compiled:?}");
    assert_eq!(interpreted.stdout, compiled.stdout);
}

#[test]
fn run_reports_parse_errors_on_stderr() {
    pebble()
        .args(["run", "examples/errors.pebble"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn parse_prints_the_syntax_tree() {
    pebble()
        .args(["parse", "examples/closures.pebble"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fn"));
}

#[test]
fn tokens_prints_one_line_per_token() {
    let output = pebble_output(&["tokens", "examples/hello.pebble"]);
    assert!(output.status.success(), "tokens failed: {output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty(), "expected at least one token line");
    assert!(
        lines[0].contains(':') && lines[0].contains(' '),
        "expected `line:col kind`, got {first:?}",
        first = lines[0]
    );
}

#[test]
fn quiet_suppresses_the_final_value() {
    pebble()
        .args(["run", "--quiet", "examples/hello.pebble"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("hello, pebble!").and(predicate::str::contains("nil").not()),
        );
}

#[test]
fn check_is_silent_on_a_valid_file() {
    pebble()
        .args(["check", "examples/fib.pebble"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn unknown_subcommands_are_usage_errors() {
    pebble().arg("frobnicate").assert().code(2);
}
