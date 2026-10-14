use pebble_nostd::bump::BumpAllocator;
use std::alloc::{GlobalAlloc, Layout};
use std::thread;

static HEAP: BumpAllocator<{ 1024 * 1024 }> = BumpAllocator::new();

fn main() {
    let layout = Layout::from_size_align(16, 8).unwrap();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            thread::spawn(move || {
                // SAFETY: `layout` is valid and the pointers are only compared.
                (0..1000)
                    .map(|_| unsafe { HEAP.alloc(layout) } as usize)
                    .collect::<Vec<usize>>()
            })
        })
        .collect();
    let mut all: Vec<usize> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
    all.sort_unstable();
    let before = all.len();
    all.dedup();
    println!("{} allocations, {} distinct addresses", before, all.len());
    println!("closest pair is {} bytes apart", all.windows(2).map(|w| w[1] - w[0]).min().unwrap());
    println!("bytes_used = {}", HEAP.bytes_used());
}
