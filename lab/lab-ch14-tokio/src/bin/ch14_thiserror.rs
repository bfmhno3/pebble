use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
enum FrameError {
    #[error("i/o error on the connection")]
    Io(#[from] io::Error),
    #[error("frame length {length} exceeds maximum frame size {max}")]
    TooLarge { length: usize, max: usize },
    #[error("frame payload is not valid UTF-8")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("peer closed the connection without a response")]
    Closed,
}

fn main() {
    let error = FrameError::TooLarge { length: 1024, max: 16 };
    println!("{error}");
    let error = FrameError::from(String::from_utf8(vec![0xff]).unwrap_err());
    println!("{error}");
    println!("has a source: {}", std::error::Error::source(&error).is_some());
    println!("{}", FrameError::Closed);
}
