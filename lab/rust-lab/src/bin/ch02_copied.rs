fn line_start(starts: &[u32], line: u32) -> Option<u32> {
    let index = line.checked_sub(1)? as usize;
    starts.get(index)
}

fn main() {
    println!("{:?}", line_start(&[0, 3, 9], 2));
}
