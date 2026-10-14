#[derive(Copy, Clone, Debug)]
struct Offset(u32);

fn main() {
    let a = Offset(3);
    let b = Offset(4);
    println!("{:?}", a + b);
}
