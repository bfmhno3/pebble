use std::ffi::{CString, c_char};

unsafe extern "C" {
    fn strlen(s: *const c_char) -> usize;
}

fn main() {
    let s = CString::new("pebble").unwrap();
    // SAFETY: `s` is a valid NUL-terminated string that outlives the call.
    let n = unsafe { strlen(s.as_ptr()) };
    println!("strlen = {n}");
}
