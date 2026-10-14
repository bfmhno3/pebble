use std::collections::HashMap;

struct Compiler {
    pool: Vec<String>,
    names: HashMap<String, usize>,
}

impl Compiler {
    fn add(&mut self, text: &str) -> usize {
        self.pool.push(text.to_owned());
        self.pool.len() - 1
    }

    fn name(&mut self, name: &str) -> usize {
        *self
            .names
            .entry(name.to_owned())
            .or_insert_with(|| self.add(name))
    }
}

fn main() {
    let mut compiler = Compiler { pool: Vec::new(), names: HashMap::new() };
    println!("{}", compiler.name("x"));
}
