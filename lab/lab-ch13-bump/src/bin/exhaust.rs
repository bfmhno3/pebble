use pebble_nostd::bump::BumpAllocator;

#[global_allocator]
static HEAP: BumpAllocator<{ 64 * 1024 }> = BumpAllocator::new();

fn main() {
    println!("arena already used by the runtime: {} bytes", HEAP.bytes_used());
    let big = vec![0u8; 100 * 1024];
    println!("allocated {} bytes", big.len());
}
