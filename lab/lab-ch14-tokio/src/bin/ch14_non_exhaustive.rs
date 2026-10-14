use pebble_net::FrameError;

fn describe(error: &FrameError) -> &'static str {
    match error {
        FrameError::Io(_) => "io",
        FrameError::TooLarge { .. } => "too large",
        FrameError::InvalidUtf8(_) => "utf8",
        FrameError::Closed => "closed",
    }
}

fn main() {
    println!("{}", describe(&FrameError::Closed));
}
