use std::num::ParseIntError;

fn with_question(a: &str, b: &str) -> Result<i64, ParseIntError> {
    let x = a.parse::<i64>()?;
    let y = b.parse::<i64>()?;
    Ok(x + y)
}

fn by_hand(a: &str, b: &str) -> Result<i64, ParseIntError> {
    let x = match a.parse::<i64>() {
        Ok(value) => value,
        Err(error) => return Err(error),
    };
    let y = match b.parse::<i64>() {
        Ok(value) => value,
        Err(error) => return Err(error),
    };
    Ok(x + y)
}

fn main() {
    println!("{:?}", with_question("1", "2"));
    println!("{:?}", by_hand("1", "2"));
    println!("{:?}", with_question("1", "x"));
    println!("{:?}", by_hand("1", "x"));
}
