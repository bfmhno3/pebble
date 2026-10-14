#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct SourceId(u32);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Span {
    source: SourceId,
    start: u32,
    end: u32,
}

impl Span {
    fn intersect(self, other: Span) -> Option<Span> {
        if self.source != other.source {
            return None;
        }
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end {
            Some(Span { source: self.source, start, end })
        } else {
            None
        }
    }
}

fn main() {
    let s = SourceId(0);
    let a = Span { source: s, start: 2, end: 8 };
    let b = Span { source: s, start: 5, end: 12 };
    let c = Span { source: s, start: 9, end: 12 };
    println!("{:?}", a.intersect(b));
    println!("{:?}", a.intersect(c));
}
