fn main() {
    let line_starts: Vec<u32> = vec![0, 3, 9, 20];
    let offset = 10;

    let at_or_before = |&start: &u32| start <= offset;
    println!("{}", line_starts.partition_point(at_or_before));
    println!("{}", line_starts.partition_point(|start| *start <= offset));
    println!("{}", line_starts.partition_point(|&start| start <= 2));
}
