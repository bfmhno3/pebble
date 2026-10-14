#[cfg(target_os = "linux")]
fn platform() -> &'static str {
    "linux"
}

#[cfg(target_os = "windows")]
fn platform() -> &'static str {
    "windows"
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn platform() -> &'static str {
    "something else"
}

fn main() {
    println!("compiled for {}", platform());
    println!("cfg! is an ordinary bool: {}", cfg!(target_os = "linux"));
    println!("debug assertions: {}", cfg!(debug_assertions));
    println!("pointer width: {}", if cfg!(target_pointer_width = "64") { 64 } else { 32 });
}
