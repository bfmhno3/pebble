use std::mem::size_of;

#[allow(dead_code)]
enum Op {
    Const(u32),
    Nil,
    Pop,
}

#[allow(dead_code)]
#[repr(C)]
struct CInstruction {
    op: u8,
    arg: u32,
}

fn main() {
    println!("enum Op        = {} bytes", size_of::<Op>());
    println!("Option<Op>     = {} bytes", size_of::<Option<Op>>());
    println!("repr(C) struct = {} bytes", size_of::<CInstruction>());
}
