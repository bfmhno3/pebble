fn add_one(x: u32) -> u32 {
    x + 1
}

fn main() {
    let block_value = {
        let a = 2;
        a * 10 // no semicolon: the block evaluates to this
    };
    let unit = {
        let a = 2;
        a * 10;
    };
    println!("{}", add_one(1));
    println!("{block_value}");
    println!("{unit:?}");
}
