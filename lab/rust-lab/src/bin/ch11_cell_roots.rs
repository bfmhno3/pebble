// Second attempt: no recursion (a worklist), mark bits still a plain Vec<bool>.
// Marking now compiles on its own. Trouble starts at the caller, which picks
// roots by looking at the heap it is about to mark.
#[derive(Clone, Copy)]
struct GcRef(u32);

trait GcObject {
    fn trace(&self, mark: &mut dyn FnMut(GcRef));
}

struct Heap<O: GcObject> {
    slots: Vec<Option<O>>,
    marked: Vec<bool>,
}

impl<O: GcObject> Heap<O> {
    fn mark(&mut self, root: GcRef) {
        let mut pending = vec![root];
        while let Some(current) = pending.pop() {
            let index = current.0 as usize;
            let Some(object) = self.slots[index].as_ref() else { continue };
            if std::mem::replace(&mut self.marked[index], true) {
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
    let mut heap = Heap {
        slots: vec![Some(Node { pinned: true, children: vec![] })],
        marked: vec![false],
    };
    // Every pinned object is a root.
    for (r, node) in heap.iter() {
        if node.pinned {
            heap.mark(r);
        }
    }
}
