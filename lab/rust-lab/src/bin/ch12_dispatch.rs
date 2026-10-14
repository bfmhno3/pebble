use std::mem::size_of;

trait Engine {
    fn name(&self) -> &'static str;
    fn run(&mut self) -> u32;
}

struct Fast(u32);
struct Slow;

impl Engine for Fast {
    fn name(&self) -> &'static str {
        "fast"
    }
    fn run(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
}

impl Engine for Slow {
    fn name(&self) -> &'static str {
        "slow"
    }
    fn run(&mut self) -> u32 {
        0
    }
}

// Static dispatch: the compiler writes one copy per concrete type.
fn run_static<E: Engine>(engine: &mut E) -> u32 {
    engine.run()
}

// Dynamic dispatch: one copy, the call goes through a vtable.
fn run_dynamic(engine: &mut dyn Engine) -> u32 {
    engine.run()
}

fn main() {
    let mut fast = Fast(41);
    let mut slow = Slow;
    println!("{} -> {}", fast.name(), run_static(&mut fast));
    println!("{} -> {}", slow.name(), run_static(&mut slow));

    let mut engines: Vec<Box<dyn Engine>> = vec![Box::new(Fast(0)), Box::new(Slow)];
    for engine in &mut engines {
        println!("{} -> {}", engine.name(), run_dynamic(engine.as_mut()));
    }

    println!("size_of::<&Fast>()       = {}", size_of::<&Fast>());
    println!("size_of::<&dyn Engine>() = {}", size_of::<&dyn Engine>());
    println!("size_of::<Fast>()        = {}", size_of::<Fast>());
    println!("size_of::<Slow>()        = {}", size_of::<Slow>());
}
