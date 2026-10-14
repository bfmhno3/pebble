use std::path::{Path, PathBuf};

fn describe(path: &Path) -> String {
    format!("{} ({} components)", path.display(), path.components().count())
}

fn shout(text: &str) -> String {
    text.to_uppercase()
}

fn main() {
    let owned: PathBuf = PathBuf::from("examples/fib.pebble");
    // `&PathBuf` where `&Path` is expected: the compiler inserts the deref.
    println!("{}", describe(&owned));

    let name = String::from("pebble");
    // Same trick: `&String` becomes `&str`.
    println!("{}", shout(&name));
}
