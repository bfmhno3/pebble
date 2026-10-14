struct Countdown {
    n: u32,
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        Some(self.n + 1)
    }
}

fn main() {
    let mut countdown = Countdown { n: 5 };
    let first_two: Vec<u32> = countdown.by_ref().take(2).collect();
    println!("first_two = {first_two:?}");
    let rest: Vec<u32> = countdown.collect();
    println!("rest      = {rest:?}");

    let mut by_value = Countdown { n: 5 };
    let _ = (&mut by_value).take(2).count();
    println!("left over = {:?}", by_value.next());
}
