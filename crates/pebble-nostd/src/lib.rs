#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
//! Pebble on bare metal.
//!
//! This crate demonstrates that the front end of the toolchain is not welded
//! to an operating system:
//!
//! 1. [`eval`] runs the *same* [`pebble_lexer`] used by the rest of the
//!    compiler, then evaluates integer arithmetic with nothing but `core` and
//!    `alloc`.
//! 2. [`bump`] provides a real [`core::alloc::GlobalAlloc`] backed by a static
//!    arena, so the evaluator's `Vec`s have somewhere to live on a target that
//!    has no `std` allocator.
//!
//! The crate is `#![no_std]` except while testing. Under `cargo test` the test
//! harness needs `std`, so the attribute is conditional; the library itself
//! never references `std`.
//!
//! Unlike the other front-end crates this one cannot `#![forbid(unsafe_code)]`:
//! implementing a [`GlobalAlloc`](core::alloc::GlobalAlloc) is inherently
//! unsafe. Instead we localize the unsafe code to [`bump`] and require every
//! unsafe operation to sit in an explicit `unsafe` block via
//! `#![deny(unsafe_op_in_unsafe_fn)]`.

extern crate alloc;

pub mod bump;
pub mod eval;
