//! Runs the `no_std` evaluator on top of the bump allocator.
//!
//! The whole program, including the standard library's own allocations, is
//! served from one 1 MiB static arena: `#[global_allocator]` replaces the
//! system allocator for every `Vec`, `String`, and `Box` in the process.
//!
//! ```text
//! cargo run -p pebble-nostd --example bump_global
//! ```

use pebble_nostd::bump::BumpAllocator;

#[global_allocator]
static HEAP: BumpAllocator<{ 1024 * 1024 }> = BumpAllocator::new();

fn main() {
    let before = HEAP.bytes_used();
    let result = pebble_nostd::eval::eval_source("(1 + 2) * 3 - 4 / 2");
    let after = HEAP.bytes_used();

    println!("result = {result:?}");
    println!(
        "the evaluator allocated {} bytes from the bump arena",
        after - before
    );
}
