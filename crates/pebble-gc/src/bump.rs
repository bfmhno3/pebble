//! A tiny bump (arena) allocator built on raw pointers.
//!
//! The arena owns one flat byte buffer and hands out slices from the front by
//! advancing an offset. Individual allocations can never be freed; the whole
//! arena is rewound at once with [`BumpArena::reset`]. In exchange, allocation
//! is a bounds check, a pointer bump and a `memcpy`, and deallocation is free.
//!
//! # Alignment
//!
//! Every allocation starts at the arena's current offset, which is only
//! advanced by whole bytes. The effective alignment is therefore
//! `align_of::<u8>()`, i.e. 1, which is exactly what byte strings and
//! consequently `str` need. This arena is not suitable for values with a
//! stricter alignment without adding an alignment-aware bump.

use core::mem::MaybeUninit;
use core::ptr;
use core::slice;

/// A bump arena: a fixed-size byte buffer handed out from the front.
///
/// Returned references borrow the arena mutably, so the borrow checker
/// guarantees that a second allocation cannot invalidate an outstanding
/// reference and that [`BumpArena::reset`] cannot run while one is alive.
pub struct BumpArena {
    buffer: Vec<MaybeUninit<u8>>,
    offset: usize,
}

impl BumpArena {
    /// Creates an arena owning `bytes` of storage.
    pub fn with_capacity(bytes: usize) -> Self {
        Self {
            buffer: vec![MaybeUninit::uninit(); bytes],
            offset: 0,
        }
    }

    /// Copies `s` into the arena and returns the copy.
    ///
    /// # Panics
    ///
    /// Panics if the arena does not have enough free space left.
    pub fn alloc_str(&mut self, s: &str) -> &mut str {
        let bytes = self.alloc_bytes(s.as_bytes());
        // SAFETY: `alloc_bytes` copied the exact bytes of `s`, which is valid
        // UTF-8, so the returned byte slice is still valid UTF-8.
        unsafe { core::str::from_utf8_unchecked_mut(bytes) }
    }

    /// Copies `bytes` into the arena and returns the copy.
    ///
    /// # Panics
    ///
    /// Panics if the arena does not have enough free space left.
    pub fn alloc_bytes(&mut self, bytes: &[u8]) -> &mut [u8] {
        let start = self.offset;
        let end = start
            .checked_add(bytes.len())
            .expect("bump arena size overflowed usize");
        assert!(
            end <= self.buffer.len(),
            "bump arena capacity of {} bytes exceeded",
            self.buffer.len()
        );
        self.offset = end;

        // SAFETY: `start..end` was just checked to be inside `self.buffer`, and
        // `MaybeUninit<u8>` has the same layout and alignment (1) as `u8`, so
        // the cast keeps the pointer properly aligned and in bounds.
        // `copy_nonoverlapping` is sound because `bytes` is an external slice
        // that cannot alias the arena. The returned range is not handed out
        // again: `offset` only moves forward, and `reset` needs `&mut self`, so
        // it cannot run while the returned reference is alive.
        unsafe {
            let base = self.buffer.as_mut_ptr().cast::<u8>();
            let destination = base.add(start);
            ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len());
            slice::from_raw_parts_mut(destination, bytes.len())
        }
    }

    /// Number of bytes handed out so far.
    pub fn used(&self) -> usize {
        self.offset
    }

    /// Total number of bytes the arena owns.
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Rewinds the arena, freeing every allocation at once.
    ///
    /// The bytes are not zeroed, and previously returned references cannot be
    /// used afterwards because `reset` borrows the arena mutably.
    pub fn reset(&mut self) {
        self.offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::BumpArena;

    #[test]
    fn allocates_str_and_bytes() {
        let mut arena = BumpArena::with_capacity(64);
        {
            let text = arena.alloc_str("hello");
            assert_eq!(&*text, "hello");
            text.make_ascii_uppercase();
            assert_eq!(&*text, "HELLO");
        }
        {
            let bytes = arena.alloc_bytes(&[1, 2, 3]);
            assert_eq!(bytes, &[1, 2, 3]);
        }

        assert_eq!(arena.used(), 8);
        assert_eq!(arena.capacity(), 64);
    }

    #[test]
    fn reset_reuses_storage() {
        let mut arena = BumpArena::with_capacity(16);
        let start = arena.alloc_bytes(b"0123456789").as_ptr() as usize;
        assert_eq!(arena.used(), 10);

        arena.reset();
        assert_eq!(arena.used(), 0);

        let again = arena.alloc_bytes(b"abcdefghij");
        assert_eq!(again, b"abcdefghij");
        assert_eq!(again.as_ptr() as usize, start, "storage is reused verbatim");
    }

    #[test]
    fn allocations_do_not_overlap() {
        let mut arena = BumpArena::with_capacity(32);
        let (first_start, first_len) = {
            let first = arena.alloc_bytes(b"aaaa");
            (first.as_ptr() as usize, first.len())
        };
        let (second_start, second_len) = {
            let second = arena.alloc_str("bbbbbb");
            (second.as_ptr() as usize, second.len())
        };

        let first_end = first_start + first_len;
        let second_end = second_start + second_len;
        assert!(
            first_end <= second_start || second_end <= first_start,
            "allocations overlap: {first_start}..{first_end} vs {second_start}..{second_end}"
        );
    }

    #[test]
    #[should_panic(expected = "capacity")]
    fn over_capacity_panics() {
        let mut arena = BumpArena::with_capacity(4);
        arena.alloc_bytes(b"12345");
    }
}
