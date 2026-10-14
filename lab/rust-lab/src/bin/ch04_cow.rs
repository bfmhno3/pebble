use std::borrow::Cow;

fn unescape_newlines(raw: &str) -> Cow<'_, str> {
    if raw.contains("\\n") {
        Cow::Owned(raw.replace("\\n", "\n"))
    } else {
        Cow::Borrowed(raw)
    }
}

fn main() {
    let plain = unescape_newlines("plain");
    let escaped = unescape_newlines("a\\nb");
    for (label, cow) in [("plain", &plain), ("escaped", &escaped)] {
        let kind = match cow {
            Cow::Borrowed(_) => "Borrowed",
            Cow::Owned(_) => "Owned",
        };
        println!("{label:8} {kind:9} {cow:?}");
    }
    println!("size_of::<&str>()       = {}", size_of::<&str>());
    println!("size_of::<String>()     = {}", size_of::<String>());
    println!("size_of::<Cow<str>>()   = {}", size_of::<Cow<'static, str>>());
}
