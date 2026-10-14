#[derive(Debug)]
struct Tok<'src> {
    text: &'src str,
}

fn pick<'a>(a: Tok<'a>, b: Tok<'a>) -> Tok<'a> {
    if a.text.len() >= b.text.len() { a } else { b }
}

fn main() {
    let global: Tok<'static> = Tok { text: "global" };
    let local = String::from("local-and-longer");
    let borrowed = Tok { text: &local };
    println!("{:?}", pick(global, borrowed));
}
