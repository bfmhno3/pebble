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
    let countdown = Countdown { n: 5 };
    let first_two: Vec<u32> = countdown.take(2).collect();
    let rest: Vec<u32> = countdown.collect();
    println!("{first_two:?} {rest:?}");
}
