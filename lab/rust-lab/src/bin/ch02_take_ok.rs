use std::mem;

struct Holder {
    name: String,
}

impl Holder {
    fn steal(&mut self) -> String {
        mem::take(&mut self.name)
    }
}

fn main() {
    let mut holder = Holder { name: String::from("a.pebble") };
    println!("stolen = {:?}", holder.steal());
    println!("left   = {:?}", holder.name);

    let old = mem::replace(&mut holder.name, String::from("b.pebble"));
    println!("old = {old:?}, new = {:?}", holder.name);

    let mut other = String::from("c.pebble");
    mem::swap(&mut holder.name, &mut other);
    println!("holder = {:?}, other = {other:?}", holder.name);
}
