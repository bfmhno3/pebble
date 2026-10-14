use std::collections::HashMap;

struct Compiler {
    pool: Vec<String>,
    names: HashMap<String, usize>,
}

impl Compiler {
    fn name(&mut self, name: &str) -> usize {
        *self
            .names
            .entry(name.to_owned())
            .or_insert_with(|| {
                self.pool.push(name.to_owned());
                self.pool.len() - 1
            })
    }
}

fn main() {
    let mut compiler = Compiler { pool: Vec::new(), names: HashMap::new() };
    println!("{} {} {}", compiler.name("x"), compiler.name("y"), compiler.name("x"));
    println!("pool = {:?}", compiler.pool);
}
