// The arena's promise, tested against the borrow checker: a second allocation
// while the first reference is alive must not compile.
struct SafeArena {
    buffer: Vec<u8>,
    offset: usize,
}

impl SafeArena {
    fn alloc_bytes(&mut self, bytes: &[u8]) -> &mut [u8] {
        let start = self.offset;
        let end = start + bytes.len();
        self.offset = end;
        let slot = &mut self.buffer[start..end];
        slot.copy_from_slice(bytes);
        slot
    }
}

fn main() {
    let mut arena = SafeArena { buffer: vec![0; 16], offset: 0 };
    let a = arena.alloc_bytes(b"one");
    let b = arena.alloc_bytes(b"two");
    println!("{a:?} {b:?}");
}
