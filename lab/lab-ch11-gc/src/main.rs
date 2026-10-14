// An index handle can dangle without any memory unsafety: it just points at
// whatever lives in that slot now.
use pebble_gc::{GcObject, GcRef, Heap};

#[derive(Debug)]
struct Name(&'static str);

impl GcObject for Name {
    fn trace(&self, _mark: &mut dyn FnMut(GcRef)) {}
}

fn main() {
    let mut heap: Heap<Name> = Heap::new();
    let old = heap.alloc(Name("first"));
    heap.free(old);
    println!("alive after free: {}", heap.is_alive(old));

    let new = heap.alloc(Name("second"));
    println!("old == new: {}", old == new);
    println!("old now reads as {:?}", heap.get(old).0);
}
