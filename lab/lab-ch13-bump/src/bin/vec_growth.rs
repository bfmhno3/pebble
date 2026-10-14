use pebble_nostd::bump::BumpAllocator;

#[global_allocator]
static HEAP: BumpAllocator<{ 1024 * 1024 }> = BumpAllocator::new();

fn main() {
    let start = HEAP.bytes_used();
    let mut v: Vec<u64> = Vec::new();
    let mut last_capacity = 0;
    for i in 0..1000 {
        v.push(i);
        if v.capacity() != last_capacity {
            last_capacity = v.capacity();
            println!(
                "len {:>4}: capacity {:>4}, arena used {:>6}",
                v.len(),
                last_capacity,
                HEAP.bytes_used() - start
            );
        }
    }
    println!("live data: {} bytes", v.len() * 8);
}
