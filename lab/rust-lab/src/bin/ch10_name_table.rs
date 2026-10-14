#[derive(Clone, Copy)]
enum Opcode {
    Const(u32),
    Nil,
    Pop,
    Add,
    Mul,
    Return,
}

impl Opcode {
    fn name(self) -> &'static str {
        match self {
            Opcode::Const(_) => "Const",
            Opcode::Nil => "Nil",
            Opcode::Pop => "Pop",
            Opcode::Add => "Add",
            Opcode::Mul => "Mul",
        }
    }
}

fn main() {
    println!("{}", Opcode::Return.name());
}
