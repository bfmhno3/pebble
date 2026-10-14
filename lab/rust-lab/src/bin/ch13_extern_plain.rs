use std::ffi::{CString, c_char};

extern "C" {
    fn strlen(s: *const c_char) -> usize;
}

fn main() {
    let s = CString::new("pebble").unwrap();
    let n = unsafe { strlen(s.as_ptr()) };
    println!("strlen = {n}");
}
