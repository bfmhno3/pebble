fn digits_while(mut n: u32) -> usize {
    let mut count = 1;
    while n >= 10 {
        n /= 10;
        count += 1;
    }
    count
}

fn digits_loop(mut n: u32) -> usize {
    let mut count = 1;
    loop {
        if n < 10 {
            break count;
        }
        n /= 10;
        count += 1;
    }
}

fn digits_ilog(n: u32) -> usize {
    n.checked_ilog10().map_or(1, |log| log as usize + 1)
}

fn main() {
    for n in [0, 9, 10, 99, 100, 12345, u32::MAX] {
        println!(
            "{n:>10}: while={} loop={} ilog={}",
            digits_while(n),
            digits_loop(n),
            digits_ilog(n)
        );
        assert_eq!(digits_while(n), digits_loop(n));
        assert_eq!(digits_while(n), digits_ilog(n));
    }
}
