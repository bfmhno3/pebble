#[derive(Debug)]
struct Label {
    start: u32,
    message: Option<String>,
}

fn main() {
    let line_no = 7;
    let width = 4;
    println!("[{line_no}]");
    println!("[{line_no:>width$}]");
    println!("[{line_no:<width$}]");
    println!("[{line_no:0>width$}]");
    println!("[{:>1$}]", line_no, 6);
    let label = Label { start: 4, message: Some("found `;`".to_string()) };
    println!("{label:?}");
    println!("{label:#?}");
    println!("{}", 255);
    println!("{:x} {:#x} {:b}", 255, 255, 5);
    println!("{:.3}", 3.14159);
}
