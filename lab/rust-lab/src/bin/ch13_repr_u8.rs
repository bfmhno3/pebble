use std::mem::size_of;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Status {
    Ok = 0,
    Full = 1,
    Closed = 7,
}

impl TryFrom<u8> for Status {
    type Error = u8;

    fn try_from(raw: u8) -> Result<Self, u8> {
        match raw {
            0 => Ok(Status::Ok),
            1 => Ok(Status::Full),
            7 => Ok(Status::Closed),
            other => Err(other),
        }
    }
}

fn main() {
    println!("size_of::<Status>() = {}", size_of::<Status>());
    println!("Closed as u8 = {}", Status::Closed as u8);
    println!("{:?}", Status::try_from(1));
    println!("{:?}", Status::try_from(3));
}
