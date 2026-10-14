// The same arena without unsafe, to see what unsafe buys.
struct SafeArena {
    buffer: Vec<u8>,
    offset: usize,
}

impl SafeArena {
    fn with_capacity(bytes: usize) -> Self {
        Self { buffer: vec![0; bytes], offset: 0 }
    }

    fn alloc_bytes(&mut self, bytes: &[u8]) -> &mut [u8] {
        let start = self.offset;
        let end = start + bytes.len();
        self.offset = end;
        let slot = &mut self.buffer[start..end];
        slot.copy_from_slice(bytes);
        slot
    }

    fn alloc_str(&mut self, s: &str) -> &mut str {
        let bytes = self.alloc_bytes(s.as_bytes());
        std::str::from_utf8_mut(bytes).expect("copied from a &str")
    }
}

fn main() {
    let mut arena = SafeArena::with_capacity(16);
    let a = arena.alloc_str("peb");
    a.make_ascii_uppercase();
    println!("{a}");
    let b = arena.alloc_str("ble");
    println!("{b}");
}
