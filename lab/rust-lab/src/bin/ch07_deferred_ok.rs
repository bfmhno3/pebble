fn main() {
    let args = std::env::args().count();
    let label;
    if args > 1 {
        label = "with arguments";
    } else {
        label = "no arguments";
    }
    println!("{label}");
}
