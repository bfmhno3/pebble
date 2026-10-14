use std::ops::{Add, AddAssign, Index, Sub};

#[derive(Copy, Clone, Debug, PartialEq)]
struct Offset(u32);

impl Add for Offset {
    type Output = Offset;
    fn add(self, other: Offset) -> Offset {
        Offset(self.0 + other.0)
    }
}

impl Add<u32> for Offset {
    type Output = Offset;
    fn add(self, bytes: u32) -> Offset {
        Offset(self.0 + bytes)
    }
}

impl Sub for Offset {
    type Output = u32;
    fn sub(self, other: Offset) -> u32 {
        self.0.saturating_sub(other.0)
    }
}

impl AddAssign<u32> for Offset {
    fn add_assign(&mut self, bytes: u32) {
        self.0 += bytes;
    }
}

struct Names(Vec<String>);

impl Index<Offset> for Names {
    type Output = String;
    fn index(&self, at: Offset) -> &String {
        &self.0[at.0 as usize]
    }
}

fn main() {
    let a = Offset(3);
    let b = Offset(4);
    let mut c = a + b;
    c += 10;
    println!("{:?} {:?} {}", a + b, a + 1, c - a);

    let names = Names(vec!["a.pebble".to_string(), "b.pebble".to_string()]);
    println!("{}", names[Offset(1)]);
    println!("{}", names[Offset(9)]);
}
