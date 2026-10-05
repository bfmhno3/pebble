## What and why

<!-- One or two sentences. Link the issue this closes, if any. -->

## How

<!-- A short summary of the approach, if it is not obvious from the diff. -->

## Checklist

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] If this changes the language, both backends (interpreter and VM) agree and a
      test proves it
- [ ] Docs (`README.md`, `docs/ARCHITECTURE.md`, `CHANGELOG.md`) updated if needed
