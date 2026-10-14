use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let a: Rc<RefCell<Vec<i64>>> = Rc::new(RefCell::new(Vec::new()));
    let b = Rc::clone(&a);
    println!("strong count: {}", Rc::strong_count(&a));
    b.borrow_mut().push(1);
    a.borrow_mut().push(2);
    println!("a = {:?}", a.borrow());
    println!("b = {:?}", b.borrow());
    println!("same allocation: {}", Rc::ptr_eq(&a, &b));
    drop(b);
    println!("strong count after drop(b): {}", Rc::strong_count(&a));
}
