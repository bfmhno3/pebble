// Exercise 1: a generation counter makes stale handles detectable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct GcRef {
    index: u32,
    generation: u32,
}

struct Slot<O> {
    generation: u32,
    object: Option<O>,
}

struct Heap<O> {
    slots: Vec<Slot<O>>,
    free: Vec<u32>,
}

impl<O> Heap<O> {
    fn new() -> Self {
        Self { slots: Vec::new(), free: Vec::new() }
    }

    fn alloc(&mut self, object: O) -> GcRef {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.object = Some(object);
            GcRef { index, generation: slot.generation }
        } else {
            self.slots.push(Slot { generation: 0, object: Some(object) });
            GcRef { index: self.slots.len() as u32 - 1, generation: 0 }
        }
    }

    fn free(&mut self, r: GcRef) {
        let slot = &mut self.slots[r.index as usize];
        assert_eq!(slot.generation, r.generation, "stale handle");
        slot.object = None;
        slot.generation += 1;
        self.free.push(r.index);
    }

    fn get(&self, r: GcRef) -> Option<&O> {
        let slot = self.slots.get(r.index as usize)?;
        if slot.generation == r.generation { slot.object.as_ref() } else { None }
    }
}

fn main() {
    let mut heap = Heap::new();
    let old = heap.alloc("first");
    heap.free(old);
    let new = heap.alloc("second");
    println!("old == new: {}", old == new);
    println!("old reads as {:?}", heap.get(old));
    println!("new reads as {:?}", heap.get(new));
    println!("size_of::<GcRef>() = {}", std::mem::size_of::<GcRef>());
}
