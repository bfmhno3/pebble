fn trim(s: &str) -> &str {
    s.trim()
}

fn apply(f: fn(&str) -> &str, input: &str) -> usize {
    f(input).len()
}

fn apply_explicit(f: for<'a> fn(&'a str) -> &'a str, input: &str) -> usize {
    f(input).len()
}

fn main() {
    let owned = String::from("  padded  ");
    println!("{}", apply(trim, &owned));
    println!("{}", apply_explicit(trim, "  other  "));
}
