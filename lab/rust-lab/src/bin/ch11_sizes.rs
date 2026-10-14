use std::mem::size_of;

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct GcRef(u32);

#[allow(dead_code)]
enum Object {
    Str(String),
    List(Vec<GcRef>),
    Closure { name: String, captured: Vec<GcRef> },
}

fn main() {
    println!("GcRef                 = {} bytes", size_of::<GcRef>());
    println!("*const u8             = {} bytes", size_of::<*const u8>());
    println!("Object                = {} bytes", size_of::<Object>());
    println!("Option<Object>        = {} bytes", size_of::<Option<Object>>());
    println!("Cell<bool>            = {} bytes", size_of::<std::cell::Cell<bool>>());
    println!("Option<Box<Object>>   = {} bytes", size_of::<Option<Box<Object>>>());
}
