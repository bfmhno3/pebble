#[derive(Copy, Clone, Debug)]
struct Pt {
    x: u32,
}

#[derive(Debug)]
struct Name {
    len: u32,
}

fn show_pt(p: Pt) {
    println!("{p:?}");
}

fn show_name(n: Name) {
    println!("{n:?}");
}

fn main() {
    let p = Pt { x: 1 };
    show_pt(p);
    show_pt(p); // snippet A

    let n = Name { len: 1 };
    show_name(n);
    show_name(n); // snippet B
}
