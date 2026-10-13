# Changelog

All notable changes to Pebble are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `pebble-macros`: the `#[differential]` attribute, which turns a function
  returning a Pebble program into a test comparing both backends.
- `pebble-bench`: a differential test suite run through `#[differential]`.
- `pebble-cli`: `pebble check` accepts several files and checks them on
  scoped threads; the REPL is behind the default `repl` Cargo feature;
  REPL meta commands parse through `FromStr`.
- `pebble-nostd`: `BumpAllocator<const N: usize>` and a `bump_global` example
  that runs the evaluator on a `#[global_allocator]`.
- `pebble-net`: `FrameError`, a typed error with a `source()` chain.
- `pebble-value`: `builtin_names()`.
- `pebble-ast`: `IntoIterator for &Program`.
- `pebble-span`: `SourceMap` implements `Index<SourceId>`.

### Changed

- `pebble-span`: `SourceMap::file` is replaced by `map[id]`.
- `pebble-vm`: builtin names come from `pebble_value::builtin_names()`; the
  `BUILTINS` constant is removed.
- `pebble-interp`: the call-depth guard is `with_call_depth`, taking an
  `FnOnce`.
- `pebble-bytecode`: names are interned through a `HashMap` entry.
- `pebble-cli`: both engines sit behind one internal `Backend` trait.
- `pebble-sys`: the opaque arena type carries `PhantomData` markers.
- `pebble-nostd`: `HEAP_SIZE` is now `DEFAULT_HEAP_SIZE`.

### Fixed

- Intra-doc links that made `cargo doc` fail with `-D warnings`.

## [0.1.0]

First working release. Pebble is a complete, if small, language.

### Added

- `pebble-span`: source ids, byte spans, source maps with line/column lookup
  (`#![no_std]`).
- `pebble-diag`: diagnostics, labels, notes, and a caret renderer
  (`#![no_std]`).
- `pebble-lexer`: a zero-copy lexer with error recovery (`#![no_std]`).
- `pebble-ast`: the syntax tree plus a `Visit` trait and a pretty-printer.
- `pebble-macros`: `#[derive(Spanned)]` and the `opcode_set!` function-like
  macro.
- `pebble-parser`: recursive-descent and precedence-climbing parser with error
  recovery, producing Rust-like diagnostics.
- `pebble-value`: the runtime object model (values, environments, builtins) with
  operator overloading and a boxed runtime error type.
- `pebble-interp`: a tree-walking interpreter with closures, recursion limits,
  and control flow as data.
- `pebble-bytecode`: the compiler from AST to an opcode stream, with jump
  backpatching, a constant pool, and a disassembler.
- `pebble-gc`: a generic mark-sweep collector behind `GcObject`, plus an unsafe
  bump arena.
- `pebble-vm`: a stack-based VM with GC roots, matching the interpreter's
  observable semantics.
- `pebble-sys`: a C shim (FNV-1a, an arena, a fold with a callback) compiled by
  `cc` and wrapped safely.
- `pebble-nostd`: a `no_std` evaluator with a bump `GlobalAlloc`.
- `pebble-net`: an asynchronous remote evaluator over TCP on Tokio.
- `pebble-cli`: the `pebble` driver (`run`, `repl`, `tokens`, `parse`, `check`,
  `bench`).

[Unreleased]: https://github.com/bfmhno3/pebble/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bfmhno3/pebble/releases/tag/v0.1.0
