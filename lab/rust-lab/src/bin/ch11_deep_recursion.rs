// Why `mark` keeps a worklist: a recursive marker dies on a deep list.
use std::cell::Cell;

#[derive(Clone, Copy)]
struct GcRef(u32);

trait GcObject {
    fn trace(&self, mark: &mut dyn FnMut(GcRef));
}

struct Heap<O: GcObject> {
    slots: Vec<Option<O>>,
    marked: Vec<Cell<bool>>,
}

impl<O: GcObject> Heap<O> {
    fn mark_recursive(&self, r: GcRef) {
        let index = r.0 as usize;
        if self.marked[index].replace(true) {
            return;
        }
        self.slots[index].as_ref().unwrap().trace(&mut |child| self.mark_recursive(child));
    }
}

struct Node(Option<GcRef>);

impl GcObject for Node {
    fn trace(&self, mark: &mut dyn FnMut(GcRef)) {
        if let Some(child) = self.0 {
            mark(child);
        }
    }
}

fn run() {
    let depth: u32 = std::env::args().nth(1).map_or(10_000, |s| s.parse().unwrap());
    let mut slots = vec![Some(Node(None))];
    for i in 1..=depth {
        slots.push(Some(Node(Some(GcRef(i - 1)))));
    }
    let marked = (0..slots.len()).map(|_| Cell::new(false)).collect();
    let heap = Heap { slots, marked };
    heap.mark_recursive(GcRef(depth));
    println!("marked {} objects at depth {depth}", heap.marked.iter().filter(|m| m.get()).count());
}

fn main() {
    // `cargo test` runs each test on a thread with a 2 MiB stack; the main
    // thread usually gets 8 MiB. Pick one with the second argument.
    if std::env::args().nth(2).as_deref() == Some("test-thread") {
        std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(run).unwrap().join().unwrap();
    } else {
        run();
    }
}
