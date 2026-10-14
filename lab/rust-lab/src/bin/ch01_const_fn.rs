const fn double(x: u32) -> u32 {
    x * 2
}

fn triple(x: u32) -> u32 {
    x * 3
}

const A: u32 = double(21);
const B: u32 = triple(14);

fn main() {
    println!("{A} {B}");
}
