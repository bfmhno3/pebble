// The fix: mark bits in `Cell<bool>`, so `mark` takes `&self`.
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
    fn mark(&self, root: GcRef) {
        let mut pending = vec![root];
        while let Some(current) = pending.pop() {
            let index = current.0 as usize;
            let Some(object) = self.slots[index].as_ref() else { continue };
            if self.marked[index].replace(true) {
                continue;
            }
            object.trace(&mut |child| pending.push(child));
        }
    }

    fn iter(&self) -> impl Iterator<Item = (GcRef, &O)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|o| (GcRef(i as u32), o)))
    }
}

struct Node {
    pinned: bool,
    children: Vec<GcRef>,
}

impl GcObject for Node {
    fn trace(&self, mark: &mut dyn FnMut(GcRef)) {
        for &child in &self.children {
            mark(child);
        }
    }
}

fn main() {
    let heap = Heap {
        slots: vec![
            Some(Node { pinned: true, children: vec![GcRef(1)] }),
            Some(Node { pinned: false, children: vec![] }),
            Some(Node { pinned: false, children: vec![] }),
        ],
        marked: vec![Cell::new(false), Cell::new(false), Cell::new(false)],
    };
    for (r, node) in heap.iter() {
        if node.pinned {
            heap.mark(r);
        }
    }
    let marks: Vec<bool> = heap.marked.iter().map(Cell::get).collect();
    println!("{marks:?}");
}
