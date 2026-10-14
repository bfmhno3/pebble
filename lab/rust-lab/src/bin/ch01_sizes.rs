#[derive(Copy, Clone)]
struct SourceId(u32);

#[derive(Copy, Clone)]
struct Span {
    source: SourceId,
    start: u32,
    end: u32,
}

fn main() {
    println!("size_of::<SourceId>() = {}", std::mem::size_of::<SourceId>());
    println!("size_of::<Span>()     = {}", std::mem::size_of::<Span>());
    println!("size_of::<usize>()    = {}", std::mem::size_of::<usize>());
    let s = Span { source: SourceId(0), start: 2, end: 9 };
    println!("{} {} {}", s.source.0, s.start, s.end);
}
