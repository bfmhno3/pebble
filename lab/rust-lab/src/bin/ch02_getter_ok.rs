struct File {
    name: String,
}

impl File {
    fn name(&self) -> &str {
        &self.name
    }

    // what the compiler fills in for you (lifetimes are chapter 4)
    fn name_explicit<'a>(&'a self) -> &'a str {
        &self.name
    }
}

fn main() {
    let file = File { name: String::from("a.pebble") };
    let name = file.name();
    println!("{name} {}", file.name_explicit());
}
