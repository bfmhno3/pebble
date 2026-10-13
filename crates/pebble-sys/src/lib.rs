//! FFI shims: a small hand-written C library linked into the Pebble runtime.
//!
//! The C code lives in `csrc/` and is compiled by `build.rs` with the [`cc`]
//! crate into a static library that this crate links. Three things are
//! demonstrated here:
//!
//! 1. Value-in/value-out calls ([`fnv1a`]).
//! 2. Ownership across the boundary: [`Arena`] owns a C heap allocation and
//!    frees it in [`Drop`].
//! 3. A callback the other way: [`fold`] hands a Rust closure to C as a
//!    `void *` context, and C calls back into a Rust `extern "C"` trampoline.
//!
//! # Soundness of the wrappers
//!
//! * No Rust type mirrors the layout of `PebbleArena`; it is an opaque
//!   [`NonNull`] handle produced only by `pebble_arena_create`.
//! * [`Arena::alloc`] returns a slice borrowed from `&mut self`. Because the
//!   borrow is exclusive, only one slice can be alive at a time, so a later
//!   `alloc` cannot hand out memory overlapping a slice that a caller still
//!   holds. (The C allocator is a bump allocator, so regions handed out by
//!   successive calls are in fact disjoint anyway; the exclusive borrow makes
//!   the guarantee legible to the borrow checker instead of relying on the
//!   caller.)
//! * [`Arena`] is `!Send + !Sync` because it holds a [`NonNull`], which is
//!   correct: the C arena has no internal synchronisation.
//! * [`fold`] catches panics inside its trampoline. Unwinding across an
//!   `extern "C"` boundary is undefined behaviour, so a panicking closure never
//!   propagates out of the callback; the payload is stashed and the panic is
//!   resumed in Rust once C has returned.
//!
//! Ownership never crosses the boundary in both directions at once: the C
//! arena is allocated by C and freed by C through [`Drop`], the closure passed
//! to [`fold`] stays owned by Rust, and C only ever borrows it through the
//! context pointer for the duration of the call.

use std::any::Any;
use std::ffi::c_void;
use std::marker::{PhantomData, PhantomPinned};
use std::ops::RangeInclusive;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::ptr::NonNull;
use std::slice;

/// Opaque handle for the C arena type.
///
/// The layout is deliberately unknown: Rust only ever names this type inside a
/// pointer, and the C side of the header is the single source of truth for what
/// it actually contains.
///
/// The marker follows the Rustonomicon's pattern for opaque FFI types: the raw
/// pointer makes the type `!Send` and `!Sync`, and `PhantomPinned` makes it
/// `!Unpin`, so safe code can never move or share it by accident.
#[repr(C)]
struct PebbleArena {
    /// Makes the type impossible to construct in Rust.
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

/// The callback signature `pebble_fold` calls for every value.
type FoldFn = unsafe extern "C" fn(i64, i64, *mut c_void) -> i64;

unsafe extern "C" {
    /// Reads `len` bytes at `data`; `data` may be null when `len` is 0.
    fn pebble_fnv1a(data: *const u8, len: usize) -> u64;

    /// Returns null on allocation failure.
    fn pebble_arena_create(capacity: usize) -> *mut PebbleArena;
    /// Returns null when the request does not fit.
    fn pebble_arena_alloc(arena: *mut PebbleArena, len: usize) -> *mut c_void;
    fn pebble_arena_used(arena: *const PebbleArena) -> usize;
    fn pebble_arena_capacity(arena: *const PebbleArena) -> usize;
    /// Invalidates every pointer the arena has handed out.
    fn pebble_arena_reset(arena: *mut PebbleArena);
    /// Frees the arena and its backing storage.
    fn pebble_arena_destroy(arena: *mut PebbleArena);

    /// Calls `cb` once for every integer in `[lo, hi]`, in order.
    fn pebble_fold(init: i64, lo: i64, hi: i64, cb: FoldFn, ctx: *mut c_void) -> i64;
}

/// Computes the 64-bit FNV-1a hash of `data` using the C implementation.
///
/// ```
/// # use pebble_sys::fnv1a;
/// let digest = fnv1a(b"hello");
/// assert_eq!(digest, 0xa430_d846_80aa_bd0b);
/// ```
#[must_use]
pub fn fnv1a(data: &[u8]) -> u64 {
    // SAFETY: `data.as_ptr()` is valid for `data.len()` reads for the duration
    // of the call and the C function only reads through it. A shared slice
    // never yields a null pointer, and `len == 0` is explicitly allowed.
    unsafe { pebble_fnv1a(data.as_ptr(), data.len()) }
}

/// An owned C bump allocator.
///
/// Created by [`Arena::with_capacity`] and destroyed (C `free`) when the value
/// is dropped. The arena is `!Send + !Sync`; the C struct has no locking.
///
/// At most one slice returned by [`Arena::alloc`] may be alive at a time,
/// because it borrows the arena exclusively. Call [`Arena::reset`] to make all
/// of the capacity available again, which invalidates previously handed out
/// slices.
#[derive(Debug)]
pub struct Arena {
    ptr: NonNull<PebbleArena>,
}

impl Arena {
    /// Creates an arena with room for `bytes` bytes of allocations.
    ///
    /// Returns `None` if the C side cannot obtain memory. A capacity of zero is
    /// valid: the returned arena exists and every [`Arena::alloc`] fails.
    #[must_use]
    pub fn with_capacity(bytes: usize) -> Option<Arena> {
        // SAFETY: `pebble_arena_create` has no preconditions and returns either
        // a live arena owned by the caller or null.
        let ptr = unsafe { pebble_arena_create(bytes) };
        Some(Arena {
            ptr: NonNull::new(ptr)?,
        })
    }

    /// Bump-allocates `len` uninitialised bytes and returns them as a slice.
    ///
    /// The returned slice borrows the arena exclusively, so no other allocation
    /// (and no other method of this arena) can be used while it is alive. This
    /// is what makes the returned `&mut [u8]` sound even though the C allocator
    /// hands out raw pointers: Rust cannot observe two live regions at once.
    ///
    /// Returns `None` when the remaining capacity cannot satisfy the request
    /// (including any request on a zero-capacity arena).
    #[must_use]
    pub fn alloc(&mut self, len: usize) -> Option<&mut [u8]> {
        // SAFETY: `self.ptr` is a live arena owned by `self`, and the C function
        // neither retains the arena nor frees it.
        let raw = unsafe { pebble_arena_alloc(self.ptr.as_ptr(), len) };
        let raw = NonNull::new(raw)?.cast::<u8>();
        // SAFETY: `raw` points into the arena's storage and the C allocator
        // guarantees at least `len` bytes there, aligned for any scalar. The
        // region is not aliased: any other borrow of `self` would have to go
        // through this `&mut self` borrow. The pointer is never null.
        Some(unsafe { slice::from_raw_parts_mut(raw.as_ptr(), len) })
    }

    /// Bytes handed out so far, including alignment padding between allocations.
    #[must_use]
    pub fn used(&self) -> usize {
        // SAFETY: `self.ptr` is a live arena and the C function only reads it.
        unsafe { pebble_arena_used(self.ptr.as_ptr()) }
    }

    /// Total bytes this arena can hand out.
    #[must_use]
    pub fn capacity(&self) -> usize {
        // SAFETY: `self.ptr` is a live arena and the C function only reads it.
        unsafe { pebble_arena_capacity(self.ptr.as_ptr()) }
    }

    /// Rewinds the arena, making all of its capacity available again.
    ///
    /// Every slice previously returned by [`Arena::alloc`] is invalidated, which
    /// the exclusive `&mut self` borrow already enforces.
    pub fn reset(&mut self) {
        // SAFETY: `self.ptr` is a live arena. The C function only updates the
        // bump offset; `&mut self` guarantees no outstanding slice.
        unsafe { pebble_arena_reset(self.ptr.as_ptr()) };
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        // SAFETY: `self.ptr` came from `pebble_arena_create` and has not been
        // destroyed yet (this is the only place that destroys it), so the C
        // function receives a valid, uniquely owned arena.
        unsafe { pebble_arena_destroy(self.ptr.as_ptr()) };
    }
}

/// State shared between [`fold`] and its C trampoline.
struct FoldCtx<'a> {
    /// The caller's closure.
    f: &'a mut dyn FnMut(i64, i64) -> i64,
    /// Set by the trampoline if the closure panicked.
    panic: Option<Box<dyn Any + Send + 'static>>,
}

/// The Rust function C calls once per value, cast to `pebble_fold_fn`.
///
/// A panic must not unwind into C, so the closure runs inside
/// [`catch_unwind`]; the payload is stashed in the context and the panic is
/// resumed in [`fold`] after C has returned.
extern "C" fn fold_trampoline(acc: i64, value: i64, ctx: *mut c_void) -> i64 {
    // SAFETY: `fold` passes a pointer to a `FoldCtx` that outlives the whole
    // `pebble_fold` call, passes the same pointer to every invocation, and the
    // trampoline is the only callback C is given for that call.
    let ctx: &mut FoldCtx<'_> = unsafe { &mut *ctx.cast() };
    if ctx.panic.is_some() {
        // The closure already panicked; do not call into possibly-broken state
        // again while C keeps walking the range.
        return acc;
    }
    match catch_unwind(AssertUnwindSafe(|| (ctx.f)(acc, value))) {
        Ok(next) => next,
        Err(payload) => {
            ctx.panic = Some(payload);
            acc
        }
    }
}

/// Folds `range` through `f` inside C, returning the final accumulator.
///
/// C calls `f(acc, value)` once per integer in `range`, in ascending order, with
/// `init` as the initial accumulator; each call receives the previous call's
/// result. An empty range (`start > end`) returns `init` without invoking `f`.
///
/// The closure is boxed and passed to C as an opaque context pointer, so it may
/// capture state mutably.
///
/// # Panics
///
/// A panic in `f` is caught at the FFI boundary and resumed on this side once C
/// has unwound its own frames: unwinding through C would be undefined
/// behaviour. The panic payload is preserved.
///
/// ```
/// # use pebble_sys::fold;
/// assert_eq!(fold(0, 1..=100, |acc, value| acc + value), 5050);
/// ```
pub fn fold(init: i64, range: RangeInclusive<i64>, f: impl FnMut(i64, i64) -> i64) -> i64 {
    let mut f: Box<dyn FnMut(i64, i64) -> i64> = Box::new(f);
    let mut ctx = FoldCtx {
        f: &mut *f,
        panic: None,
    };
    // SAFETY: `fold_trampoline` is a valid `pebble_fold_fn` and `ctx` outlives
    // the call. C invokes the callback only while `pebble_fold` runs, only with
    // `ctx` as given, and retains nothing after it returns. `ctx` is not shared:
    // C is single-threaded here and the range walk is sequential.
    let result = unsafe {
        pebble_fold(
            init,
            *range.start(),
            *range.end(),
            fold_trampoline,
            std::ptr::from_mut(&mut ctx).cast::<c_void>(),
        )
    };

    if let Some(payload) = ctx.panic.take() {
        // `ctx` is no longer borrowed by anything, and C has returned, so it is
        // safe to unwind now with the original payload.
        resume_unwind(payload);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_empty_is_offset_basis() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    fn fnv1a_hello_matches_reference() {
        assert_eq!(fnv1a(b"hello"), 0xa430_d846_80aa_bd0b);
    }

    #[test]
    fn fnv1a_foobar_matches_reference() {
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn fnv1a_long_input_matches_rust_reference() {
        fn reference(data: &[u8]) -> u64 {
            data.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
            })
        }

        let data: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        assert_eq!(fnv1a(&data), reference(&data));
    }

    #[test]
    fn arena_reports_capacity() {
        let arena = Arena::with_capacity(64).expect("arena creation failed");
        assert_eq!(arena.capacity(), 64);
        assert_eq!(arena.used(), 0);
    }

    #[test]
    fn arena_alloc_used_and_reset() {
        let mut arena = Arena::with_capacity(64).expect("arena creation failed");

        let region = arena.alloc(8).expect("first allocation failed");
        assert_eq!(region.len(), 8);
        region.copy_from_slice(&[7u8; 8]);
        assert_eq!(region, &[7u8; 8]);

        let used_after_first = arena.used();
        assert!(used_after_first >= 8, "used = {used_after_first}");

        let _second = arena.alloc(8).expect("second allocation failed");
        assert!(arena.used() > used_after_first);

        arena.reset();
        assert_eq!(arena.used(), 0);

        // After a reset the whole capacity is available again.
        let whole = arena.alloc(64).expect("allocation after reset failed");
        assert_eq!(whole.len(), 64);
        assert_eq!(arena.used(), 64);
    }

    #[test]
    fn arena_allocations_do_not_overlap() {
        let mut arena = Arena::with_capacity(256).expect("arena creation failed");

        let first = arena.alloc(37).expect("first allocation failed");
        let first_start = first.as_ptr() as usize;
        let first_end = first_start + first.len();

        let second = arena.alloc(37).expect("second allocation failed");
        let second_start = second.as_ptr() as usize;
        let second_end = second_start + second.len();

        assert!(
            first_end <= second_start || second_end <= first_start,
            "regions overlap: {first_start:#x}..{first_end:#x} and \
             {second_start:#x}..{second_end:#x}"
        );
    }

    #[test]
    fn arena_exhaustion_returns_none() {
        let mut arena = Arena::with_capacity(16).expect("arena creation failed");
        assert!(arena.alloc(16).is_some());
        assert!(arena.alloc(1).is_none());
    }

    #[test]
    fn zero_capacity_arena_allocation_fails() {
        let mut arena = Arena::with_capacity(0).expect("arena creation failed");
        assert_eq!(arena.capacity(), 0);
        assert!(arena.alloc(1).is_none());
    }

    #[test]
    fn fold_sums_inclusive_range() {
        assert_eq!(fold(0, 1..=100, |acc, value| acc + value), 5050);
    }

    #[test]
    // An empty range is a valid input for `fold`, not a typo: C must return
    // `init` untouched, so the literal reversed bounds are intentional.
    #[allow(clippy::reversed_empty_ranges)]
    fn fold_empty_range_returns_init() {
        assert_eq!(fold(42, 5..=4, |acc, _| acc + 1), 42);
    }

    #[test]
    fn fold_visits_every_value_in_order() {
        let mut seen = Vec::new();
        let total = fold(0, -2..=2, |acc, value| {
            seen.push(value);
            acc + value
        });
        assert_eq!(seen, vec![-2, -1, 0, 1, 2]);
        assert_eq!(total, 0);
    }

    #[test]
    fn fold_closure_can_capture_mutable_state() {
        let mut calls = 0;
        let result = fold(1, 0..=5, |acc, value| {
            calls += 1;
            acc * (value + 1)
        });
        assert_eq!(calls, 6);
        assert_eq!(result, 720);
    }

    #[test]
    fn fold_handles_int64_max_bound() {
        assert_eq!(
            fold(0, i64::MAX..=i64::MAX, |acc, value| acc + value),
            i64::MAX
        );
    }

    #[test]
    #[should_panic(expected = "closure exploded")]
    fn fold_resumes_closure_panic() {
        let _ = fold(0, 1..=10, |acc, value| {
            assert_ne!(value, 3, "closure exploded");
            acc + value
        });
    }
}
