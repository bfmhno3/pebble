fn register(name: impl Into<String>) -> usize {
    let owned: String = name.into();
    owned.len()
}

fn main() {
    println!("{}", register(42));
}
