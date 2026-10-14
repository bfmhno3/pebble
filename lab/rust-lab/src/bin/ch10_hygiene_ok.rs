macro_rules! double {
    ($e:expr) => {{
        let x = $e;
        x + x
    }};
}

fn main() {
    let x = 10;
    // The macro's `x` and ours are different variables.
    println!("{}", double!(x + 1));
    println!("{x}");
}
