unsafe fn first_byte(p: *const u8) -> u8 {
    *p
}

fn main() {
    let bytes = [7u8, 8, 9];
    println!("{}", unsafe { first_byte(bytes.as_ptr()) });
}
