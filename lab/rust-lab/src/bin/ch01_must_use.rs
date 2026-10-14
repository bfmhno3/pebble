struct Span {
    start: u32,
    end: u32,
}

impl Span {
    #[must_use]
    const fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }
}

fn main() {
    let s = Span { start: 2, end: 9 };
    s.len();
}
