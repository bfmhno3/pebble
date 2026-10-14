use std::mem::size_of;
use std::ptr::NonNull;

fn main() {
    println!("*mut u8             {}", size_of::<*mut u8>());
    println!("NonNull<u8>         {}", size_of::<NonNull<u8>>());
    println!("Option<NonNull<u8>> {}", size_of::<Option<NonNull<u8>>>());
    println!("Option<*mut u8>     {}", size_of::<Option<*mut u8>>());
    println!("Option<&u8>         {}", size_of::<Option<&u8>>());
    println!("Option<u32>         {}", size_of::<Option<u32>>());
}
