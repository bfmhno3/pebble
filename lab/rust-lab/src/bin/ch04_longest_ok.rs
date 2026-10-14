fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

fn main() {
    let outer = String::from("abc");
    let inner = String::from("ab");
    println!("{}", longest(&outer, &inner));
}
