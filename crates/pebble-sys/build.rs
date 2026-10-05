//! Compiles the C shim in `csrc/` into a static library linked into this crate.

/// Source files that make up the C shim, relative to this crate's root.
const C_SOURCES: &[&str] = &["csrc/pebble_shim.c", "csrc/include/pebble_shim.h"];

fn main() {
    for source in C_SOURCES {
        println!("cargo:rerun-if-changed={source}");
    }

    cc::Build::new()
        .file("csrc/pebble_shim.c")
        .include("csrc/include")
        .flag_if_supported("-std=c99")
        .warnings(true)
        .compile("pebble_shim");
}
