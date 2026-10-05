//! A precise mark-sweep garbage collector plus a small bump arena for one-shot
//! allocations ([`bump`]).
//!
//! The collector is generic over the [`GcObject`] trait, so an embedder such as
//! the Pebble VM plugs in its own object enum and decides how each object
//! reports its child references. The heap owns a flat table of slots; an
//! embedder allocates with [`Heap::alloc`] and keeps the returned [`GcRef`]
//! handles in its own roots (value stack, globals, temporaries). Collection is
//! split into two embedder-driven phases:
//!
//! 1. Mark every root: `heap.mark(root)` for each handle the embedder still
//!    holds. Marking follows the children reported by [`GcObject::trace`]
//!    transitively.
//! 2. Sweep: `heap.collect()` returns every unmarked slot to the free list and
//!    clears the mark bits, ready for the next cycle.
//!
//! [`Heap::mark`] only needs `&self`, so roots can be marked through a shared
//! reference; the mark bits live in [`Cell`]s inside the heap. There is no
//! global collector state and no `Drop`-based collection: reclamation happens
//! only when the embedder calls [`Heap::collect`].
//!
//! # Example
//!
//! ```
//! use pebble_gc::{GcObject, GcRef, Heap};
//!
//! /// A list-like object: it traces the references it holds.
//! struct Node(Vec<GcRef>);
//!
//! impl GcObject for Node {
//!     fn trace(&self, mark: &mut dyn FnMut(GcRef)) {
//!         for &child in &self.0 {
//!             mark(child);
//!         }
//!     }
//! }
//!
//! let mut heap: Heap<Node> = Heap::new();
//! let kept = heap.alloc(Node(vec![]));
//! let garbage = heap.alloc(Node(vec![]));
//!
//! heap.mark(kept);
//! heap.collect();
//!
//! assert_eq!(heap.live_count(), 1);
//! assert!(heap.is_alive(kept));
//! assert!(!heap.is_alive(garbage));
//! ```

use core::cell::Cell;

pub mod bump;

/// A handle to a live object inside a [`Heap`].
///
/// `GcRef` is a plain `u32` slot index and is `Copy`, so embedders can store
/// handles on their own stacks and in registers without touching the heap. A
/// handle only means something to the [`Heap`] that produced it, and it becomes
/// *dangling* once its slot is returned by [`Heap::free`] or reclaimed by
/// [`Heap::collect`]; [`Heap::is_alive`] tests for that without panicking.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GcRef(u32);

impl GcRef {
    /// The underlying slot index of this handle.
    pub const fn index(self) -> u32 {
        self.0
    }

    /// Rebuilds a handle from a slot index.
    ///
    /// The index must have come from [`GcRef::index`]; forging one for a slot
    /// that is not live produces a dangling handle.
    pub const fn from_index(index: u32) -> Self {
        Self(index)
    }
}

/// A heap-managed object that can report its child references.
///
/// The collector calls [`GcObject::trace`] once per slot while marking, passing
/// a callback that records one child reference. An implementation must report
/// every live [`GcRef`] it owns, and nothing else: a reference it fails to
/// report may be swept while still in use, and one it invents may keep garbage
/// alive forever.
pub trait GcObject {
    /// Reports each child reference to `mark`, so the collector can mark it.
    fn trace(&self, mark: &mut dyn FnMut(GcRef));
}

/// A mark-sweep heap over a flat table of slots, generic over the object type.
///
/// Freed slots are recycled: the slot table grows to the peak number of
/// simultaneously live objects, not to the total number of allocations ever
/// performed.
#[derive(Debug)]
pub struct Heap<O: GcObject> {
    slots: Vec<Option<O>>,
    free: Vec<u32>,
    marked: Vec<Cell<bool>>,
    allocations_since_gc: usize,
}

impl<O: GcObject> Heap<O> {
    /// Creates an empty heap with no slots allocated.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            marked: Vec::new(),
            allocations_since_gc: 0,
        }
    }

    /// Allocates `object` and returns a handle to it.
    ///
    /// A slot on the free list is reused if one is available; otherwise the
    /// slot table grows by one entry.
    ///
    /// # Panics
    ///
    /// Panics if the heap would need more than `u32::MAX` slots.
    pub fn alloc(&mut self, object: O) -> GcRef {
        self.allocations_since_gc += 1;
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            debug_assert!(slot.is_none(), "free list contained an occupied slot");
            *slot = Some(object);
            self.marked[index as usize].set(false);
            GcRef(index)
        } else {
            let index = u32::try_from(self.slots.len()).expect("heap outgrew its u32 slot index");
            self.slots.push(Some(object));
            self.marked.push(Cell::new(false));
            GcRef(index)
        }
    }

    /// Returns a shared reference to the object behind `r`.
    ///
    /// # Panics
    ///
    /// Panics if `r` is out of range or points to a slot that is currently
    /// free, i.e. a dangling handle. Use [`Heap::is_alive`] to check first.
    pub fn get(&self, r: GcRef) -> &O {
        self.slots
            .get(r.0 as usize)
            .and_then(|slot| slot.as_ref())
            .unwrap_or_else(|| panic!("GcRef({}) does not point to a live object", r.0))
    }

    /// Returns a mutable reference to the object behind `r`.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Heap::get`].
    pub fn get_mut(&mut self, r: GcRef) -> &mut O {
        self.slots
            .get_mut(r.0 as usize)
            .and_then(|slot| slot.as_mut())
            .unwrap_or_else(|| panic!("GcRef({}) does not point to a live object", r.0))
    }

    /// Returns the slot behind `r` to the free list.
    ///
    /// Any mark bit on the slot is cleared, so the slot is immediately
    /// reusable by [`Heap::alloc`].
    ///
    /// # Panics
    ///
    /// Panics if `r` is out of range or already free, which usually means the
    /// embedder double-freed or used a dangling handle.
    pub fn free(&mut self, r: GcRef) {
        let index = r.0 as usize;
        assert!(
            self.slots
                .get_mut(index)
                .and_then(|slot| slot.take())
                .is_some(),
            "GcRef({}) does not point to a live object",
            r.0
        );
        self.marked[index].set(false);
        self.free.push(r.0);
    }

    /// Marks `root` and everything reachable from it, transitively.
    ///
    /// Marking is idempotent: a slot that is already marked is not traced
    /// again, so repeated roots and cycles terminate. The traversal keeps an
    /// explicit worklist instead of recursing, so arbitrarily deep object
    /// graphs cannot overflow the native stack.
    ///
    /// # Panics
    ///
    /// Panics if `root` is out of range. A handle to a freed slot is ignored,
    /// since it can no longer name a reachable object.
    pub fn mark(&self, root: GcRef) {
        let mut pending = vec![root];
        while let Some(current) = pending.pop() {
            let index = current.0 as usize;
            let Some(object) = self.slots[index].as_ref() else {
                continue;
            };
            if self.marked[index].replace(true) {
                continue;
            }
            object.trace(&mut |child| pending.push(child));
        }
    }

    /// Sweeps every slot that is not marked, then clears all mark bits.
    ///
    /// The embedder must have marked its roots (via [`Heap::mark`]) before
    /// calling this. After the sweep the heap is ready for the next mark phase.
    pub fn collect(&mut self) {
        for index in 0..self.slots.len() {
            if self.marked[index].get() {
                self.marked[index].set(false);
            } else if self.slots[index].is_some() {
                let slot = u32::try_from(index).expect("heap outgrew its u32 slot index");
                self.free(GcRef(slot));
            }
        }
        self.allocations_since_gc = 0;
    }

    /// Whether `r` names a slot that currently holds an object.
    ///
    /// Unlike [`Heap::get`], this never panics: an out-of-range or freed handle
    /// simply reports `false`.
    pub fn is_alive(&self, r: GcRef) -> bool {
        self.slots
            .get(r.0 as usize)
            .is_some_and(|slot| slot.is_some())
    }

    /// Number of occupied slots, i.e. objects not yet swept.
    pub fn live_count(&self) -> usize {
        self.slots.len() - self.free.len()
    }

    /// Number of object slots the heap can hold without reallocating its
    /// tables.
    pub fn capacity(&self) -> usize {
        self.slots.capacity()
    }

    /// Estimated total bytes occupied by the live slots.
    ///
    /// This is `live_count() * size_of::<O>()`: the inline size of each object
    /// as stored in its slot. Memory a `GcObject` owns indirectly, such as a
    /// `String` buffer or a `Vec`'s heap block, is not counted, so treat this
    /// as an estimate for deciding when to collect rather than an exact
    /// allocator figure.
    pub fn bytes_allocated(&self) -> usize {
        self.live_count() * core::mem::size_of::<O>()
    }

    /// Number of allocations performed since the last [`Heap::collect`].
    ///
    /// An embedder can use this to trigger collection after a fixed number of
    /// allocations.
    pub fn allocations_since_gc(&self) -> usize {
        self.allocations_since_gc
    }
}

impl<O: GcObject> Default for Heap<O> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{GcObject, GcRef, Heap};
    use rand::prelude::*;

    /// A concrete heap object for the tests, mirroring the shapes the Pebble
    /// runtime will plug in: strings, lists and closures.
    #[derive(Clone, PartialEq, Eq, Debug)]
    enum TestObject {
        Str(String),
        List(Vec<GcRef>),
        Closure { name: String, captured: Vec<GcRef> },
    }

    impl GcObject for TestObject {
        fn trace(&self, mark: &mut dyn FnMut(GcRef)) {
            match self {
                TestObject::Str(_) => {}
                TestObject::List(items) => {
                    for &child in items {
                        mark(child);
                    }
                }
                TestObject::Closure { captured, .. } => {
                    for &child in captured {
                        mark(child);
                    }
                }
            }
        }
    }

    fn list(heap: &mut Heap<TestObject>, children: &[GcRef]) -> GcRef {
        heap.alloc(TestObject::List(children.to_vec()))
    }

    #[test]
    fn alloc_get_and_get_mut() {
        let mut heap: Heap<TestObject> = Heap::new();
        let r = heap.alloc(TestObject::Str("pebble".into()));

        assert_eq!(heap.get(r), &TestObject::Str("pebble".into()));
        assert_eq!(heap.live_count(), 1);
        assert_eq!(heap.bytes_allocated(), core::mem::size_of::<TestObject>());
        assert!(heap.capacity() >= 1);
        assert_eq!(heap.allocations_since_gc(), 1);

        let child = heap.alloc(TestObject::Str("child".into()));
        assert_eq!(
            heap.bytes_allocated(),
            2 * core::mem::size_of::<TestObject>()
        );

        let closure = heap.alloc(TestObject::Closure {
            name: "f".into(),
            captured: vec![],
        });
        match heap.get_mut(closure) {
            TestObject::Closure { name, captured } => {
                name.push_str("@1");
                captured.push(child);
            }
            other => panic!("expected a closure, got {other:?}"),
        }
        assert_eq!(
            heap.get(closure),
            &TestObject::Closure {
                name: "f@1".into(),
                captured: vec![child],
            }
        );
        assert_eq!(r.index(), 0);
        assert_eq!(GcRef::from_index(0), r);
    }

    #[test]
    fn freed_slots_are_reused() {
        let mut heap: Heap<TestObject> = Heap::new();
        let first = heap.alloc(TestObject::Str("first".into()));
        heap.free(first);
        assert_eq!(heap.live_count(), 0);
        assert_eq!(heap.bytes_allocated(), 0);
        assert!(!heap.is_alive(first));

        let reused = heap.alloc(TestObject::Str("second".into()));
        assert_eq!(reused, first, "the freed slot should be reused");
        assert_eq!(heap.live_count(), 1);
        assert_eq!(heap.slots.len(), 1, "the slot table should not have grown");
    }

    #[test]
    fn mark_traces_children_transitively() {
        let mut heap: Heap<TestObject> = Heap::new();
        let leaf = heap.alloc(TestObject::Str("leaf".into()));
        let middle = list(&mut heap, &[leaf]);
        let root = list(&mut heap, &[middle]);
        let closure = heap.alloc(TestObject::Closure {
            name: "capture".into(),
            captured: vec![root],
        });
        let orphan = heap.alloc(TestObject::Str("orphan".into()));

        heap.mark(closure);
        heap.mark(closure); // idempotent

        assert!(heap.marked[closure.index() as usize].get());
        assert!(heap.marked[root.index() as usize].get());
        assert!(heap.marked[middle.index() as usize].get());
        assert!(heap.marked[leaf.index() as usize].get());
        assert!(!heap.marked[orphan.index() as usize].get());

        heap.collect();

        assert_eq!(heap.live_count(), 4);
        assert!(!heap.is_alive(orphan), "the orphan was swept");
        assert!(
            !heap.marked[closure.index() as usize].get(),
            "marks are cleared for the next cycle"
        );
        assert_eq!(heap.allocations_since_gc(), 0);
    }

    #[test]
    fn collect_without_roots_frees_everything() {
        let mut heap: Heap<TestObject> = Heap::new();
        for i in 0..10 {
            heap.alloc(TestObject::Str(format!("s{i}")));
        }
        assert_eq!(heap.live_count(), 10);
        let slots = heap.slots.len();

        heap.collect();

        assert_eq!(heap.live_count(), 0);
        assert_eq!(heap.bytes_allocated(), 0);
        assert_eq!(heap.slots.len(), slots, "slots stay reserved for reuse");
    }

    #[test]
    fn deeply_nested_lists_survive() {
        let mut heap: Heap<TestObject> = Heap::new();
        let mut current = heap.alloc(TestObject::Str("bottom".into()));
        for _ in 0..10_000 {
            current = list(&mut heap, &[current]);
        }

        // The worklist marker must handle this depth without recursing.
        heap.mark(current);
        heap.collect();

        assert_eq!(heap.live_count(), 10_001);
    }

    #[test]
    fn unreachable_cycles_are_collected() {
        let mut heap: Heap<TestObject> = Heap::new();
        let a = list(&mut heap, &[]);
        let b = list(&mut heap, &[a]);
        // Close the cycle: a -> b -> a. Neither is rooted.
        match heap.get_mut(a) {
            TestObject::List(items) => items.push(b),
            other => panic!("expected a list, got {other:?}"),
        }

        heap.collect();

        assert_eq!(heap.live_count(), 0, "an unreachable cycle is swept");
        assert_eq!(heap.bytes_allocated(), 0);
        assert!(!heap.is_alive(a));
    }

    #[test]
    fn is_alive_tracks_slots() {
        let mut heap: Heap<TestObject> = Heap::new();
        let r = heap.alloc(TestObject::Str("x".into()));
        assert!(heap.is_alive(r));

        heap.free(r);
        assert!(!heap.is_alive(r));
        assert!(!heap.is_alive(GcRef::from_index(999)), "out of range");

        // A freed handle is ignored by mark and never resurrected.
        heap.mark(r);
        heap.collect();
        assert_eq!(heap.live_count(), 0);
    }

    #[test]
    #[should_panic(expected = "does not point to a live object")]
    fn getting_a_freed_reference_panics() {
        let mut heap: Heap<TestObject> = Heap::new();
        let r = heap.alloc(TestObject::Str("gone".into()));
        heap.free(r);
        let _ = heap.get(r);
    }

    #[test]
    fn randomised_reachability_matches_sweep() {
        let mut rng = rand::rng();
        for _ in 0..200 {
            let n = rng.random_range(1..40usize);

            // A random directed graph. Edges may point forward or backward, so
            // cycles are common.
            let mut edges: Vec<Vec<usize>> = Vec::with_capacity(n);
            for _ in 0..n {
                let degree = rng.random_range(0..=3);
                let mut children = Vec::with_capacity(degree);
                for _ in 0..degree {
                    children.push(rng.random_range(0..n));
                }
                edges.push(children);
            }

            let mut heap: Heap<TestObject> = Heap::new();
            for children in &edges {
                let refs: Vec<GcRef> = children
                    .iter()
                    .map(|&child| GcRef::from_index(child as u32))
                    .collect();
                heap.alloc(TestObject::List(refs));
            }

            let roots: Vec<usize> = (0..n).filter(|_| rng.random_bool(0.3)).collect();
            for &root in &roots {
                heap.mark(GcRef::from_index(root as u32));
            }

            // Expected survivors: everything reachable from the roots.
            let mut reachable = vec![false; n];
            let mut stack = roots.clone();
            while let Some(node) = stack.pop() {
                if reachable[node] {
                    continue;
                }
                reachable[node] = true;
                stack.extend_from_slice(&edges[node]);
            }

            heap.collect();

            let live: Vec<usize> = (0..n).filter(|&i| heap.slots[i].is_some()).collect();
            let expected: Vec<usize> = (0..n).filter(|&i| reachable[i]).collect();
            assert_eq!(live, expected, "the sweep kept the wrong objects");
            assert_eq!(heap.live_count(), expected.len());
            for &i in &expected {
                assert!(heap.is_alive(GcRef::from_index(i as u32)));
            }
        }
    }
}
