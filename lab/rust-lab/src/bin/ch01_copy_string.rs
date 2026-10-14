#[derive(Copy, Clone)]
struct Label {
    name: String,
}

fn main() {
    let l = Label { name: String::from("x") };
    let _ = l.clone();
}
