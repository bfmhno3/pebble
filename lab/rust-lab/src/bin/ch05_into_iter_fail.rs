struct Program {
    stmts: Vec<String>,
}

impl<'a> IntoIterator for &'a Program {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.stmts.iter()
    }
}

fn main() {
    let program = Program { stmts: vec![] };
    for stmt in program {
        println!("{stmt}");
    }
}
