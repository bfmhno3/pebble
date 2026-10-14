use std::convert::TryFrom;

#[allow(dead_code)]
#[derive(Debug)]
struct Index(u32);

impl TryFrom<i64> for Index {
    type Error = String;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        u32::try_from(value)
            .map(Index)
            .map_err(|_| format!("{value} is not a valid index"))
    }
}

fn main() {
    println!("{:?}", Index::try_from(7));
    println!("{:?}", Index::try_from(-1));
    let converted: Result<Index, _> = 5_000_000_000i64.try_into();
    println!("{:?}", converted);
    println!("{:?}", "42".parse::<i64>());
    println!("{:?}", "4x2".parse::<i64>());
    println!("{:?}", u8::try_from(300i32));
}
