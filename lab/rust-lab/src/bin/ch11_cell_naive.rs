// First attempt at `mark`: recursion, with the mark bits in a plain Vec<bool>.
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
    fn get(&self, r: GcRef) -> &O {
        self.slots[r.0 as usize].as_ref().unwrap()
    }

    fn mark(&mut self, r: GcRef) {
        if self.marked[r.0 as usize] {
            return;
        }
        self.marked[r.0 as usize] = true;
        self.get(r).trace(&mut |child| self.mark(child));
    }
}

struct Node(Vec<GcRef>);

impl GcObject for Node {
    fn trace(&self, mark: &mut dyn FnMut(GcRef)) {
        for &child in &self.0 {
            mark(child);
        }
    }
}

fn main() {
    let mut heap = Heap { slots: vec![Some(Node(vec![]))], marked: vec![false] };
    heap.mark(GcRef(0));
}
