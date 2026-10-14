struct Buf<const N: usize = 4> {
    data: [u8; N],
}

fn main() {
    let big = Buf::<4096> { data: [0; 4096] };
    let small: Buf<4> = big;
    println!("{}", small.data.len());
}
