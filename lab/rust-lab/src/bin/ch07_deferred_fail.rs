fn main() {
    let args = std::env::args().count();
    let label;
    if args > 1 {
        label = "with arguments";
    }
    println!("{label}");
}
