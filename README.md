# Pebble

> A small dynamically typed scripting language and toolchain, built from scratch
> in Rust - and a tour of the Rust language disguised as a compiler.

Pebble has a hand-written lexer, a recursive-descent + precedence-climbing
parser, a tree-walking interpreter, a bytecode compiler, a stack virtual
machine, and a hand-written mark-sweep garbage collector. It is split into a
Cargo workspace where every crate exists to make a different corner of Rust do
real work: `#![no_std]` data structures, procedural macros, `Rc`/`RefCell`
object graphs, `unsafe` raw-pointer collection, FFI, `async`, and a custom
allocator.

## Quick start

```bash
nix develop        # or a system Rust >= 1.88
cargo build --workspace
```

Run a program with either backend, or open a REPL:

```bash
cargo run -p pebble-cli -- run examples/fib.pebble        # tree-walking interpreter
cargo run -p pebble-cli -- run --vm examples/fib.pebble   # bytecode VM
cargo run -p pebble-cli -- repl
cargo run -p pebble-cli -- parse examples/closures.pebble
cargo run -p pebble-cli -- check examples/fib.pebble
```

## The language

```pebble
fn fib(n) {
    if n < 2 { return n; }
    return fib(n - 1) + fib(n - 2);
}

let xs = range(10);
let total = 0;
for x in xs {
    if x % 2 == 0 { continue; }
    total = total + x;
}
print("odd sum:", total);
print("fib(20) =", fib(20));

let counter = fn() {
    let n = 0;
    return fn() { n = n + 1; return n; };
};
```

Values: `nil`, booleans, 64-bit integers, floats, strings, lists (`[...]`),
maps (`#{...}`), closures, and builtins (`print`, `len`, `type`, `str`, `int`,
`float`, `range`, `push`, `keys`, `clock`, `assert`). Indexing supports negative
indices.

## Workspace layout

| crate             | what it teaches                                                        |
| ----------------- | ---------------------------------------------------------------------- |
| `pebble-span`     | `#![no_std]`, `Copy` types, offsets, source maps                        |
| `pebble-diag`     | error types, `Display`/`Error`, rendering, generics over sinks          |
| `pebble-lexer`    | zero-copy `&str` scanning, lifetimes, `Cow`, `Iterator`                 |
| `pebble-ast`      | algebraic data types, `Box`, recursive enums, visitor traits            |
| `pebble-macros`   | `proc-macro` crates, `syn`/`quote`, derive + function-like macros       |
| `pebble-parser`   | recursive descent, Pratt parsing, `Result`/`?`, custom errors           |
| `pebble-value`    | `Rc`/`RefCell`, interior mutability, operator overloading               |
| `pebble-interp`   | environments, closures, trait objects, control-flow as data             |
| `pebble-bytecode` | bytecode chunks, opcodes, a disassembler                                |
| `pebble-gc`       | `unsafe`, raw pointers, `MaybeUninit`, a generic mark-sweep collector   |
| `pebble-vm`       | stack machine, pattern matching over opcodes, GC roots                  |
| `pebble-sys`      | `build.rs`, `cc`, `extern "C"`, `repr(C)`, FFI boundaries               |
| `pebble-nostd`    | `no_std` + `alloc`, a bump allocator, a custom `GlobalAlloc`            |
| `pebble-net`      | `async`/`await`, Tokio, `spawn_blocking`, `Send`/`Sync`                 |
| `pebble-cli`      | `clap`, `io`, an interactive REPL                                       |
| `pebble-bench`    | Criterion benchmarks comparing the two backends                         |

`docs/ARCHITECTURE.md` explains how the pieces fit together and how the two
backends are kept semantically in sync.

## Testing and benchmarks

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo bench -p pebble-bench
```

CI runs the same checks on Linux and macOS.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
