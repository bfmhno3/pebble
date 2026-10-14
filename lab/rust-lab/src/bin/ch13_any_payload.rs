use std::any::Any;
use std::panic::catch_unwind;

fn describe(payload: &Box<dyn Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        format!("&str: {text}")
    } else if let Some(text) = payload.downcast_ref::<String>() {
        format!("String: {text}")
    } else {
        "something else".to_string()
    }
}

fn main() {
    let literal = catch_unwind(|| panic!("a literal")).unwrap_err();
    let n = 3;
    let formatted = catch_unwind(|| panic!("formatted {n}")).unwrap_err();
    let custom = catch_unwind(|| std::panic::panic_any(42_i32)).unwrap_err();
    println!("{}", describe(&literal));
    println!("{}", describe(&formatted));
    println!("{}", describe(&custom));
}
