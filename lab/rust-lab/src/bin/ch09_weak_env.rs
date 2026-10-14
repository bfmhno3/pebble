// ~/rust-lab/src/bin/ch09_weak_env.rs
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Scope {
    count: i64,
}

// A closure that only holds a Weak pointer to the scope it captured.
struct WeakClosure {
    env: Weak<RefCell<Scope>>,
}

impl WeakClosure {
    fn call(&self) -> Option<i64> {
        let env = self.env.upgrade()?;
        let mut scope = env.borrow_mut();
        scope.count += 1;
        Some(scope.count)
    }
}

// Like `make_counter()` in Pebble: the call's scope is a local that nobody else owns.
fn make_counter() -> WeakClosure {
    let scope = Rc::new(RefCell::new(Scope { count: 0 }));
    WeakClosure { env: Rc::downgrade(&scope) }
} // `scope` is dropped here, the only strong pointer is gone

fn main() {
    let next = make_counter();
    println!("first call:  {:?}", next.call());
    println!("second call: {:?}", next.call());
}
