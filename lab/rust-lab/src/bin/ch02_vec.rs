fn main() {
    let starts: Vec<u32> = vec![0, 4, 9];
    let middle: &[u32] = &starts[1..];
    println!("slice = {middle:?}, len = {}", middle.len());
    println!("get(1) = {:?}, get(10) = {:?}", starts.get(1), starts.get(10));

    let mut grown: Vec<u32> = Vec::with_capacity(8);
    grown.push(0);
    grown.push(5);
    println!("len = {}, capacity = {}", grown.len(), grown.capacity());

    let index = std::hint::black_box(10);
    println!("starts[10] = {}", starts[index]);
}
