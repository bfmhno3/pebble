/// Callable from C as `int64_t pebble_add(int64_t, int64_t)`.
#[unsafe(no_mangle)]
pub extern "C" fn pebble_add(a: i64, b: i64) -> i64 {
    a + b
}

/// Not exported under a stable name: Rust mangles it.
pub fn mangled(a: i64) -> i64 {
    a * 2
}
