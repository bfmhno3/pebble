fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

fn main() {
    let outer = String::from("abc");
    let result;
    {
        let inner = String::from("ab");
        result = longest(&outer, &inner);
        println!("inside:  {result}");
    }
    println!("outside: {result}");
}
