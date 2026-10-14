use std::marker::{PhantomData, PhantomPinned};

#[repr(C)]
struct Opaque {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}
fn assert_unpin<T: Unpin>() {}

fn main() {
    assert_send::<Opaque>();
    assert_sync::<Opaque>();
    assert_unpin::<Opaque>();
}
