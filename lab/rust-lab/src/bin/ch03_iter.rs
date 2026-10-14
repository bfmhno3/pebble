#[derive(Debug)]
struct Label {
    start: u32,
    primary: bool,
}

fn main() {
    let labels = vec![
        Label { start: 30, primary: false },
        Label { start: 4, primary: true },
        Label { start: 12, primary: true },
    ];

    // borrow, don't consume
    let first_primary = labels.iter().find(|label| label.primary).map(|label| label.start);
    println!("first primary starts at {first_primary:?}");

    let mut sorted: Vec<&Label> = labels.iter().collect();
    sorted.sort_by_key(|label| label.start);
    println!("{sorted:?}");

    let carets: String = core::iter::repeat_n('^', 5).collect();
    println!("{carets}");
    println!("labels still usable: {}", labels.len());
}
