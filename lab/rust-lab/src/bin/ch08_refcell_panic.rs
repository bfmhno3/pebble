use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(vec![1, 2, 3]);
    let reader = cell.borrow();
    println!("try_borrow_mut while reading: {:?}", cell.try_borrow_mut().is_err());
    println!("first = {}", reader[0]);
    cell.borrow_mut().push(4);
    println!("unreachable");
}
