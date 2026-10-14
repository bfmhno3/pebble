fn first_power_of_two_over(limit: u32) -> u32 {
    let mut p = 1;
    while p <= limit {
        p *= 2;
    }
    p
}

fn first_power_of_two_over_loop(limit: u32) -> u32 {
    let mut p = 1;
    loop {
        if p > limit {
            break p;
        }
        p *= 2;
    }
}

fn main() {
    println!("{}", first_power_of_two_over(100));
    println!("{}", first_power_of_two_over_loop(100));
}
