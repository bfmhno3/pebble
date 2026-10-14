use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct ByContents(Rc<RefCell<Vec<i64>>>);

impl PartialEq for ByContents {
    fn eq(&self, other: &Self) -> bool {
        *self.0.borrow() == *other.0.borrow()
    }
}

impl Eq for ByContents {}

impl Hash for ByContents {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

fn main() {
    let list = Rc::new(RefCell::new(vec![1]));
    let mut map = HashMap::new();
    map.insert(ByContents(Rc::clone(&list)), "one");
    println!("before push: {:?}", map.get(&ByContents(Rc::clone(&list))));
    list.borrow_mut().push(2);
    println!("after push:  {:?}", map.get(&ByContents(Rc::clone(&list))));
    println!("entries: {}", map.len());
}
