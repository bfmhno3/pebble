use std::fmt::Display;

fn shout_generic<T: Display>(value: T) -> String {
    format!("{value}!")
}

fn shout_impl(value: impl Display) -> String {
    format!("{value}!")
}

fn main() {
    println!("{}", shout_generic(42));
    println!("{}", shout_impl("pebble"));
    println!("{}", shout_generic::<f64>(1.5));
}
