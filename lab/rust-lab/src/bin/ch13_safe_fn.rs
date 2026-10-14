use std::ffi::c_int;

unsafe extern "C" {
    // getpid takes no arguments and has no preconditions: nothing to get wrong.
    safe fn getpid() -> c_int;
}

fn main() {
    println!("getpid() > 0: {}", getpid() > 0);
}
