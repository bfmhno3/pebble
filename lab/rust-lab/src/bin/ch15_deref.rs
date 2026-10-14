use std::path::{Path, PathBuf};

fn load(path: &Path) -> usize {
    path.as_os_str().len()
}

fn shout(text: &str) -> String {
    text.to_uppercase()
}

fn total(numbers: &[i64]) -> i64 {
    numbers.iter().sum()
}

fn main() {
    let owned = PathBuf::from("examples/fib.pebble");
    println!("{}", load(&owned)); // &PathBuf where &Path is expected

    let name = String::from("pebble");
    println!("{}", shout(&name)); // &String where &str is expected

    let list = vec![1, 2, 3];
    println!("{}", total(&list)); // &Vec<i64> where &[i64] is expected

    let boxed = Box::new(String::from("boxed"));
    println!("{}", shout(&boxed)); // &Box<String> -> &String -> &str: two steps
}
