use std::ffi::c_void;

fn main() {
    let scale = 3;
    let cb: extern "C" fn(i64, i64, *mut c_void) -> i64 = |acc, value, _ctx| acc + value * scale;
    println!("{}", cb(0, 2, std::ptr::null_mut()));
}
