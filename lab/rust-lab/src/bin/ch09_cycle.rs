// ~/rust-lab/src/bin/ch09_cycle.rs
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Scope {
    name: &'static str,
    // A "function" stored in the scope that it captured.
    function: Option<Rc<Function>>,
}

struct Function {
    // Never read: the field exists only to keep the scope alive.
    #[allow(dead_code)]
    env: Rc<RefCell<Scope>>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        println!("  dropping scope `{}`", self.name);
    }
}

fn main() {
    println!("-- a scope without functions");
    let plain = Rc::new(RefCell::new(Scope { name: "plain", function: None }));
    let weak_plain: Weak<RefCell<Scope>> = Rc::downgrade(&plain);
    drop(plain);
    println!("  alive after drop: {}", weak_plain.upgrade().is_some());

    println!("-- a scope that stores a function capturing itself");
    let globals = Rc::new(RefCell::new(Scope { name: "globals", function: None }));
    let function = Rc::new(Function { env: Rc::clone(&globals) });
    globals.borrow_mut().function = Some(function);
    println!("  strong count: {}", Rc::strong_count(&globals));
    let weak_globals = Rc::downgrade(&globals);
    drop(globals);
    println!("  alive after drop: {}", weak_globals.upgrade().is_some());
}
