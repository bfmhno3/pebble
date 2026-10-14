struct File {
    name: String,
}

impl File {
    fn name(&self) -> &str {
        &self.name
    }
}

fn main() {
    let name;
    {
        let file = File { name: String::from("a.pebble") };
        name = file.name();
    }
    println!("{name}");
}
