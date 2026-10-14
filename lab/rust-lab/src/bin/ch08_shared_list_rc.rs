use std::rc::Rc;

fn main() {
    let a: Rc<Vec<i64>> = Rc::new(Vec::new());
    let b = Rc::clone(&a);
    b.push(1);
    println!("{:?}", a);
}
