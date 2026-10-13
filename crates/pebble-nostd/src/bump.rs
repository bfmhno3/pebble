//! A bump allocator over a fixed, static arena.
//!
//! A bump (or "arena") allocator never frees. [`BumpAllocator::alloc`] hands out
//! the next slice of the arena and moves a cursor forward; `dealloc` does
//! nothing. That is a real trade-off, not a placeholder: on a microcontroller
//! the entire program often runs and then reboots, so reclaiming memory
//! mid-run buys nothing and costs bookkeeping.
//!
//! There is no `static mut` here. The cursor is an [`AtomicUsize`] and the arena
//! lives behind an [`UnsafeCell`], which is exactly the interior-mutability
//! primitive designed for this. `Sync` is implemented by hand because the
//! compiler cannot see that the shared mutations are confined to the atomic
//! cursor.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The arena size, in bytes, of a [`BumpAllocator`] that does not name one.
pub const DEFAULT_HEAP_SIZE: usize = 4096;

/// The mutable cursor of a bump allocator.
///
/// Kept as one [`UnsafeCell`] so the allocator has interior mutability without
/// making the whole [`BumpAllocator`] `mut`.
struct BumpState {
    /// Bytes handed out so far, measured from the start of the arena.
    used: AtomicUsize,
}

/// A [`GlobalAlloc`] that bumps a cursor through a fixed-size static arena.
///
/// Each instance owns `N` bytes inline (4096 unless you say otherwise), which
/// is what makes it usable as a `static` without a separate `static mut` buffer:
///
/// ```
/// use pebble_nostd::bump::BumpAllocator;
///
/// static HEAP: BumpAllocator = BumpAllocator::new();
/// ```
///
/// Allocation is lock-free: concurrent callers race on the cursor with a
/// compare-exchange loop, and whoever wins reserves its slice exclusively.
/// Exhaustion is reported as a null pointer, as [`GlobalAlloc`] requires.
pub struct BumpAllocator<const N: usize = DEFAULT_HEAP_SIZE> {
    state: UnsafeCell<BumpState>,
    heap: UnsafeCell<[u8; N]>,
}

// SAFETY: the only shared state that is ever mutated is the atomic cursor, so
// racing `alloc` calls are well defined. Two successful allocations never
// overlap, because the atomic update reserves each slice before it is handed
// out; the caller of `alloc` receives a pointer it is free to write through.
unsafe impl<const N: usize> Sync for BumpAllocator<N> {}

impl<const N: usize> BumpAllocator<N> {
    /// Creates an allocator whose entire arena is free.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: UnsafeCell::new(BumpState {
                used: AtomicUsize::new(0),
            }),
            heap: UnsafeCell::new([0; N]),
        }
    }

    /// Returns how many bytes have been handed out so far.
    ///
    /// This is the arena high-water mark since the allocator never reuses
    /// space; it is useful for asserting that a workload fits, or for sizing an
    /// arena on a device with a known memory budget.
    #[must_use]
    pub fn bytes_used(&self) -> usize {
        // SAFETY: `state` lives as long as `self`, and reading the atomic needs
        // no unique access.
        let state = unsafe { &*self.state.get() };
        state.used.load(Ordering::Relaxed)
    }

    /// The size in bytes of the arena: the const parameter `N`.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Address of the first byte of the arena.
    fn base(&self) -> usize {
        self.heap.get().cast::<u8>() as usize
    }
}

impl<const N: usize> Default for BumpAllocator<N> {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: the implementation upholds the `GlobalAlloc` contract.
//
// - `alloc` returns null on exhaustion and otherwise a pointer aligned to
//   `layout.align()` that is not aliased by any other live allocation.
// - The returned pointer is valid for writes of `layout.size()` bytes, since the
//   arena is an owned `[u8; N]` and the cursor stays within it.
// - `dealloc` is a no-op, which is permitted: freeing is optional.
unsafe impl<const N: usize> GlobalAlloc for BumpAllocator<N> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let base = self.base();
        let align = layout.align();
        let size = layout.size();
        // SAFETY: `state` outlives `self` and the closure only touches the
        // atomic cursor, so it needs no exclusive access.
        let state = unsafe { &*self.state.get() };

        // Reserve our slice by atomically advancing the cursor. `fetch_update`
        // retries until either the update succeeds or the cursor cannot grow
        // enough for this request, in which case it yields `Err` and we report
        // exhaustion.
        let reserved = state
            .used
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                let cursor = base.checked_add(used)?;
                let aligned = cursor.checked_add(align - 1)? & !(align - 1);
                let offset = aligned - base;
                let end = offset.checked_add(size)?;
                (end <= N).then_some(end)
            });

        match reserved {
            // `Err` carries the (unchanged) current cursor: no room left.
            Err(_) => ptr::null_mut(),
            // `Ok(previous)` is the cursor we reserved from. Recomputing the
            // aligned address is infallible here because the reservation above
            // already succeeded with these same values.
            Ok(previous) => {
                let aligned = (base + previous + align - 1) & !(align - 1);
                aligned as *mut u8
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // A bump allocator cannot free an individual allocation: the cursor
        // only moves forward, and there is no free list to recover a hole. The
        // whole arena is reclaimed at once when the allocator is dropped or the
        // program is reset. This is intentional, not an oversight.
    }
}

#[cfg(test)]
mod tests {
    use super::{BumpAllocator, DEFAULT_HEAP_SIZE};
    use core::alloc::{GlobalAlloc, Layout};

    static ALLOC: BumpAllocator = BumpAllocator::new();

    #[test]
    fn distinct_layouts_do_not_overlap() {
        // SAFETY: the layouts are valid (`size` is a multiple of `align`, both
        // powers of two), and a null result is handled below.
        let a = unsafe { ALLOC.alloc(Layout::from_size_align(64, 8).unwrap()) };
        let b = unsafe { ALLOC.alloc(Layout::from_size_align(128, 16).unwrap()) };
        assert!(!a.is_null() && !b.is_null(), "allocations must succeed");
        assert_eq!(a as usize % 8, 0, "first block must honour alignment");
        assert_eq!(b as usize % 16, 0, "second block must honour alignment");
        // The ranges [a, a+64) and [b, b+128) must be disjoint.
        let (a, b) = (a as usize, b as usize);
        assert!(
            a + 64 <= b || b + 128 <= a,
            "allocations at {a:#x} and {b:#x} overlap"
        );
        assert!(ALLOC.bytes_used() >= 64 + 128);
    }

    #[test]
    fn exhaustion_returns_null() {
        // A fresh allocator so this test cannot be perturbed by the other one.
        static SMALL: BumpAllocator<64> = BumpAllocator::new();
        let mut handed_out = 0;
        loop {
            // SAFETY: `Layout::new::<u8>()` is valid; the returned pointer is
            // only inspected, never written.
            let p = unsafe { SMALL.alloc(Layout::new::<u8>()) };
            if p.is_null() {
                break;
            }
            handed_out += 1;
            assert!(handed_out <= 64, "allocator overran its arena");
        }
        assert_eq!(handed_out, 64, "one byte at a time fills the arena");
        assert_eq!(SMALL.bytes_used(), 64);
    }

    #[test]
    fn dealloc_is_a_no_op() {
        static HEAP: BumpAllocator = BumpAllocator::new();
        // SAFETY: `alloc` returns a pointer that `dealloc` may be called on;
        // deallocating a bump allocation is a documented no-op.
        let p = unsafe { HEAP.alloc(Layout::new::<u64>()) };
        assert!(!p.is_null());
        let used = HEAP.bytes_used();
        unsafe { HEAP.dealloc(p, Layout::new::<u64>()) };
        assert_eq!(HEAP.bytes_used(), used, "dealloc must not move the cursor");
    }

    #[test]
    fn capacity_is_the_const_parameter() {
        assert_eq!(BumpAllocator::<128>::new().capacity(), 128);
        let default: BumpAllocator = BumpAllocator::new();
        assert_eq!(default.capacity(), DEFAULT_HEAP_SIZE);
    }
}
