use std::mem::size_of;
use std::rc::Rc;

fn main() {
    let owned: String = String::from("pebble");
    let shared: Rc<str> = Rc::from(owned.as_str());
    let other = Rc::clone(&shared);
    println!("size_of::<String>():   {}", size_of::<String>());
    println!("size_of::<Rc<str>>():  {}", size_of::<Rc<str>>());
    println!("size_of::<Rc<String>>(): {}", size_of::<Rc<String>>());
    println!("len = {}, same = {}, count = {}", shared.len(), Rc::ptr_eq(&shared, &other), Rc::strong_count(&shared));
    let upper: &str = &shared;
    println!("{}", upper.to_uppercase());
}
