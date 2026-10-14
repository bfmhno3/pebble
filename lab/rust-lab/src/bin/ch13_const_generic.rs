use std::mem::size_of;

struct Buf<const N: usize = 4> {
    data: [u8; N],
}

impl<const N: usize> Buf<N> {
    const fn new() -> Self {
        Self { data: [0; N] }
    }

    const fn capacity(&self) -> usize {
        self.data.len()
    }
}

fn main() {
    let default: Buf = Buf::new();
    let big = Buf::<4096>::new();
    println!("default: capacity {} size_of {}", default.capacity(), size_of::<Buf>());
    println!("4096:    capacity {} size_of {}", big.capacity(), size_of::<Buf<4096>>());
}
