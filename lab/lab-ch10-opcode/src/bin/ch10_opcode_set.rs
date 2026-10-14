use pebble_macros::opcode_set;

opcode_set! {
    pub enum Tiny {
        Const(u32),
        Nil,
        Jump(u32),
    }
}

fn main() {
    for op in [Tiny::Const(7), Tiny::Nil, Tiny::Jump(3)] {
        println!("{:?} is called {}", op, op.name());
    }
}
