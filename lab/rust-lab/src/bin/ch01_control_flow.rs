fn main() {
    // `if` is an expression.
    let n = 7;
    let kind = if n % 2 == 0 { "even" } else { "odd" };
    println!("{n} is {kind}");

    // `loop` can break with a value.
    let mut tries = 0;
    let first_square_over_50 = loop {
        tries += 1;
        if tries * tries > 50 {
            break tries * tries;
        }
    };
    println!("first square over 50: {first_square_over_50}");

    // `while` and `for` over a range (end exclusive).
    let mut i = 0;
    while i < 3 {
        print!("{i} ");
        i += 1;
    }
    println!();
    for j in 0..3 {
        print!("{j} ");
    }
    println!();
    for j in 0..=3 {
        print!("{j} ");
    }
    println!();

    // Labeled break leaves the outer loop.
    'outer: for row in 0..5 {
        for col in 0..5 {
            if row * col == 6 {
                println!("found {row} * {col} == 6");
                break 'outer;
            }
        }
    }
}
