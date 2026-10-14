use std::{ptr, slice};

fn main() {
    let mut storage = [0u8; 8];
    let base: *mut u8 = storage.as_mut_ptr();
    println!("null? {}  null_mut? {}", base.is_null(), ptr::null_mut::<u8>().is_null());

    let source = b"pebble";
    // SAFETY: both ranges are in bounds (6 <= 8) and do not overlap.
    let view = unsafe {
        ptr::copy_nonoverlapping(source.as_ptr(), base.add(1), source.len());
        slice::from_raw_parts_mut(base.add(1), source.len())
    };
    view.make_ascii_uppercase();
    println!("{view:?}");
    println!("{}", std::str::from_utf8(view).unwrap());
    println!("{storage:?}");

    // `str::from_utf8` checks; the unchecked version trusts you.
    let bad = vec![0xffu8, 0xfe];
    println!("checked: {:?}", std::str::from_utf8(&bad));
}
