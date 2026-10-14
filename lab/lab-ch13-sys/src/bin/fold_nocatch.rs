use pebble_sys as _;
use std::ffi::c_void;

unsafe extern "C" {
    fn pebble_fold(
        init: i64,
        lo: i64,
        hi: i64,
        cb: extern "C" fn(i64, i64, *mut c_void) -> i64,
        ctx: *mut c_void,
    ) -> i64;
}

extern "C" fn exploding(acc: i64, value: i64, _ctx: *mut c_void) -> i64 {
    assert_ne!(value, 3, "callback exploded");
    acc + value
}

fn main() {
    // SAFETY: `exploding` ignores its context, which may therefore be null.
    let total = unsafe { pebble_fold(0, 1, 5, exploding, std::ptr::null_mut()) };
    println!("total = {total}");
}
