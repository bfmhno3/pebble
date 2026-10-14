fn largest<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

fn main() {
    println!("{}", largest(3_i32, 9));
    println!("{}", largest(2.5_f64, 1.5));
    println!("{}", largest("pebble", "stone"));
}
