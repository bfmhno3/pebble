// ~/rust-lab/src/bin/ch09_iter_mutate.rs
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let list = Rc::new(RefCell::new(vec![1, 2, 3]));

    // The snapshot way: clone the Vec, loop over the copy, mutate the original.
    let snapshot = list.borrow().clone();
    for x in snapshot {
        list.borrow_mut().push(x * 10);
    }
    println!("with a snapshot: {:?}", list.borrow());

    // The tempting way: loop over the live borrow while the body mutates.
    let list = Rc::new(RefCell::new(vec![1, 2, 3]));
    for x in list.borrow().iter() {
        list.borrow_mut().push(x * 10);
    }
    println!("unreachable: {:?}", list.borrow());
}
