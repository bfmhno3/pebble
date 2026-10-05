# Contributing to Pebble

Thanks for taking a look. Pebble is a learning project, but it is kept to a
production standard: everything in `main` builds, lints, and tests clean.

## Getting a toolchain

The easiest path is Nix:

```bash
nix develop
```

Otherwise you need Rust 1.88 or newer with `rustfmt` and `clippy`.

## Before you open a pull request

Run the same checks CI runs:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

`cargo clippy ... -- -D warnings` also lints the workspace's path dependencies,
so a warning anywhere is a failure everywhere. Fix it where it lives.

## Code style

- `rustfmt` decides formatting; do not hand-format against it. `cargo fmt --all`
  before committing.
- Every public item gets a `///` doc comment.
- Keep `unsafe` small and local, and put a `// SAFETY:` comment above every
  unsafe operation explaining the invariant it relies on.
- Prefer plain, boring code over clever code. This repo is meant to be read.

## Tests

- Unit tests live next to the code and cover behaviour, boundaries, and error
  paths.
- Integration tests that drive the binary live in `crates/pebble-cli/tests/`.
- The two backends must agree. If you add a language feature, add a test that
  runs it on both the interpreter and the VM.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
feat(lexer): support hexadecimal integer literals
fix(vm): keep the GC root set in sync when popping frames
docs: correct the calling convention in ARCHITECTURE.md
```

Keep commits atomic: one logical change each.

## Adding a language feature

See the checklist at the end of `docs/ARCHITECTURE.md`. In short: lexer, AST,
parser, interpreter, bytecode compiler, VM, and a test proving both backends
agree.
