struct Parser {
    tokens: Vec<u32>,
    current: usize,
}

impl Parser {
    fn previous(&self) -> u32 {
        self.tokens[self.current.saturating_sub(1)]
    }

    fn if_statement(&mut self, start: u32) -> u32 {
        self.current += 1;
        start + self.previous()
    }
}

fn main() {
    // `v.push(v.len())`: the receiver is borrowed mutably, the argument reads `v`.
    let mut v = vec![1, 2, 3];
    v.push(v.len());
    println!("{v:?}");

    let mut parser = Parser { tokens: vec![10, 20, 30], current: 1 };
    let r = parser.if_statement(parser.previous());
    println!("{r}");
}
