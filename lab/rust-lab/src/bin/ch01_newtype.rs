#[derive(Copy, Clone)]
struct SourceId(u32);

#[derive(Copy, Clone)]
struct Span {
    source: SourceId,
    start: u32,
    end: u32,
}

impl Span {
    const fn new(source: SourceId, start: u32, end: u32) -> Self {
        Self { source, start, end }
    }
}

fn main() {
    println!("size_of::<SourceId>() = {}", std::mem::size_of::<SourceId>());
    println!("size_of::<Span>()     = {}", std::mem::size_of::<Span>());
    let ok = Span::new(SourceId(0), 2, 9);
    let bad = Span::new(0, 2, 9);
    println!("{} {}", ok.end, bad.end);
}
