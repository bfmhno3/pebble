# Lab: experiments from "用 Pebble 从零学 Rust"

This directory holds every small experiment the tutorial series asks you to
run. It is **not** part of the Cargo workspace in the repository root: each
crate below has its own empty `[workspace]` table, so `cargo test --workspace`
at the root never sees it. Build output goes to `lab/**/target`, which is
git-ignored.

## The main lab: `rust-lab/`

One crate, one file per experiment: `src/bin/chNN_<topic>.rs`.

```bash
cd lab/rust-lab
cargo run --quiet --bin ch01_bindings     # an experiment that runs
cargo build --bin ch01_casts              # a compile-fail demo
```

Many files are **meant to fail**: they show a compiler error (for example
`E0308` or `E0382`) that the post then explains. For those, `cargo build`
exiting with an error and printing the code the post quotes is the expected
result. A few files panic on purpose (overflow, `RefCell` double borrow); the
post says which. The file contents are kept terse on purpose and are not
rustfmt-formatted.

## Side crates

Each side crate depends on the **final** Pebble crates through relative paths
(`../../crates/<name>`), i.e. the code at the `tutorial-v1` tag. Run them from
inside their own directory (`cd lab/<crate>`).

| directory | chapter | what it is | run |
| --- | --- | --- | --- |
| `lab-ch03-ex/` | 3 | exercise answers for `pebble-diag` (a `Severity::Help` rendering test, a counting `DiagnosticSink`); tests only | `cargo test` |
| `lab-ch04-stages/` | 4 | the lexer cursor grown in a stage (`stage1`) and a hidden-lifetime lint demo (`hidden`) | `cargo run --quiet --bin stage1` |
| `lab-ch05-ast/` | 5 | a visitor that counts nodes (`ch05_ex_counter`), `pretty`, and `size_of` of AST types | `cargo run --quiet --bin ch05_sizes` |
| `lab-ch06-derive/` | 6 | the derive-macro experiments; the `lab-macros*` proc-macro crates live inside it | `cargo build --bin ch06_hello` |
| `lab-ch06-export/` | 6 | a proc-macro crate that exports a non-macro item (fails to build on purpose) | `cargo build` |
| `lab-ch06-nodep/` | 6 | hygiene demo: the derive's output names `::pebble_span`, which this crate does not depend on (fails on purpose) | `cargo build` |
| `lab-ch07-anyhow/` | 7 | `anyhow` context chains and `{error:#}` | `cargo run --quiet` |
| `lab-ch10-opcode/` | 10 | calling `opcode_set!` from outside the bytecode crate | `cargo run --quiet --bin ch10_opcode_set` |
| `lab-ch11-gc/` | 11 | stale handles in `pebble-gc`, and the generation-counter exercise | `cargo run --quiet --bin generation` |
| `lab-ch13-sys/` | 13 | `pebble-sys` experiments: byte arenas, `Send`, panics through C | `cargo run --quiet --bin fold_panic` |
| `lab-ch13-cdylib/` | 13 | `#[unsafe(no_mangle)] extern "C"` in a `cdylib`, plus a C caller `main.c` | `cargo build && cc main.c -Ltarget/debug -lpebble_ffi_demo -o /tmp/ffi_main && LD_LIBRARY_PATH=target/debug /tmp/ffi_main` |
| `lab-ch13-bump/` | 13 | a `#[global_allocator]` bump arena measuring `Vec` growth, `SourceMap::add`, threads, exhaustion | `cargo run --quiet --bin stages` |
| `lab-ch14-tokio/` | 14 | threads vs tasks, framing, `select!`, blocking, `thiserror` | `cargo run --quiet --bin ch14_wire` |
| `lab-ch15-tests/` | 15 | unit vs integration vs doc tests (the post breaks the doc test by changing `2` to `3` in `src/lib.rs`, and the integration test by importing the private `secret`) | `cargo test` |

Some demos in `lab-ch06-derive`, `lab-ch13-sys` and `lab-ch14-tokio` are
compile-fail on purpose, like the main lab.

### Notes on versions

The experiments were written while the crates were being rebuilt chapter by
chapter, then re-run against the final tag crates. They all still behave as the
posts describe when built against the final crates, with these notes:

- `lab-ch04-stages` prints with the *final* `pebble-span` / `pebble-diag`;
  the output is the same as in the post.
- The exercise crates below were whole-workspace copies in the original
  experiments; they are stored as patches instead.

## `exercises/`: patches against the tag's `crates/`

Experiments that modify an existing Pebble crate are stored as patches. Apply
one to a clean checkout of this repository, run the command, and revert it:

```bash
git apply lab/exercises/ch04-hex.patch
cargo test -p pebble-lexer
git apply -R lab/exercises/ch04-hex.patch
```

| patch | chapter | change | try |
| --- | --- | --- | --- |
| `ch04-elide.patch` | 4 | drop the lifetime from `tokenize`'s return type (lint warnings about hidden lifetimes) | `cargo build -p pebble-lexer` |
| `ch04-hex.patch` | 4 | `0x` hexadecimal literals via `from_str_radix` | `cargo test -p pebble-lexer` |
| `ch04-peek.patch` | 4 | `peek_second` replaced by a byte-based `peek_second_byte` | `cargo test -p pebble-lexer` |
| `ch05-sized.patch` | 5 | remove `?Sized` from the `walk_*` functions (`E0277`) | `cargo build -p pebble-ast` |
| `ch13-s2.patch` | 13 | `BumpAllocator::reset(&mut self)` and `Arena::remaining` through the C shim | `cargo test -p pebble-nostd -p pebble-sys` |
| `ch13-scratch.patch` | 13 | `Arena::alloc(&self)` (clippy `mut_from_ref`) and an `overlap` example | `cargo clippy -p pebble-sys --lib`, `cargo run -p pebble-sys --example overlap` |
| `ch15-features.patch` | 15 | tests that exist only with or without the `repl` feature | `cargo test -p pebble-cli` and `... --no-default-features` |
| `ch15-doc-link.patch` | 15 | an intra-doc link that cannot resolve | `RUSTDOCFLAGS="-D warnings" cargo doc -p pebble-value --no-deps` |

## `cpp/`

The C and C++ programs the posts compare Rust against. They are plain
single-file programs (`g++ -std=c++17 -Wall -Wextra file.cpp`, or `cc -Wall -Wextra file.c`); the C++
file `ch04_view.cpp` is meant to be run under AddressSanitizer
(`-fsanitize=address`).
