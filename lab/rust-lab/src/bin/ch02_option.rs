fn line_start(starts: &[u32], line: u32) -> Option<u32> {
    let index = line.checked_sub(1)? as usize;
    starts.get(index).copied()
}

fn main() {
    let starts = [0, 3, 9];
    println!("{:?}", line_start(&starts, 2));
    println!("{:?}", line_start(&starts, 0));
    println!("{:?}", line_start(&starts, 7));

    match line_start(&starts, 3) {
        Some(offset) => println!("line 3 starts at byte {offset}"),
        None => println!("no such line"),
    }

    let width = line_start(&starts, 9).map_or(0, |offset| offset + 1);
    println!("width = {width}");
}
