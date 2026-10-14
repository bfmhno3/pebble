macro_rules! make_x {
    () => {
        let x = 1;
    };
}

fn main() {
    make_x!();
    println!("{x}");
}
