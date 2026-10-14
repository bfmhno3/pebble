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
    let program = Program {
        stmts: vec!["let x = 1;".to_string(), "print(x);".to_string()],
    };

    for stmt in &program {
        println!("for:      {stmt}");
    }

    // what the compiler rewrites that loop into
    let mut iter = IntoIterator::into_iter(&program);
    while let Some(stmt) = iter.next() {
        println!("desugared: {stmt}");
    }
}
