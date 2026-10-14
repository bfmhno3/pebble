struct Holder {
    name: String,
}

impl Holder {
    fn steal(&mut self) -> String {
        self.name
    }
}

fn main() {
    let mut holder = Holder { name: String::from("a.pebble") };
    println!("{}", holder.steal());
}
