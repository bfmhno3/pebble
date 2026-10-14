use std::cell::{Cell, UnsafeCell};

struct CursorOnly {
    used: Cell<usize>,
}

struct Arena {
    heap: UnsafeCell<[u8; 16]>,
}

static CURSOR: CursorOnly = CursorOnly { used: Cell::new(0) };
static ARENA: Arena = Arena { heap: UnsafeCell::new([0; 16]) };

fn main() {
    CURSOR.used.set(1);
    println!("{}", ARENA.heap.get() as usize != 0);
}
