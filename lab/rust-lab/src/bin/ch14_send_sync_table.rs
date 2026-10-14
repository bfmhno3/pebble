use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

fn is_send<T: Send>() {}
fn is_sync<T: Sync>() {}

fn main() {
    is_send::<String>();
    is_sync::<String>();
    is_send::<Arc<Mutex<Vec<u32>>>>();
    is_sync::<Arc<Mutex<Vec<u32>>>>();
    is_send::<Cell<u32>>();
    is_send::<RefCell<u32>>();
    is_send::<Rc<u32>>();
    is_send::<MutexGuard<'static, u32>>();
}
