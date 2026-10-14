#[cfg(target_os = "windows")]
fn only_on_windows() -> &'static str {
    "windows"
}

fn main() {
    println!("{}", only_on_windows());
}
