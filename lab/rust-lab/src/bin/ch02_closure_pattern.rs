fn main() {
    let line_starts: Vec<u32> = vec![0, 3, 9, 20];
    let offset = 10;

    let count = line_starts.partition_point(|&start| start <= offset);
    println!("lines starting at or before {offset}: {count}");

    let bad = line_starts.partition_point(|start| start <= offset);
    println!("{bad}");
}
