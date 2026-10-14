use std::error::Error;

use pebble_net::FrameError;

fn main() {
    let utf8 = String::from_utf8(vec![0x68, 0xff, 0xfe]).unwrap_err();
    let error = FrameError::from(utf8);

    println!("display : {error}");
    println!("debug   : {error:?}");

    let mut source = error.source();
    let mut depth = 1;
    while let Some(cause) = source {
        println!("cause {depth} : {cause}");
        source = cause.source();
        depth += 1;
    }

    let wrapped = anyhow::Error::new(error).context("while reading the response from 127.0.0.1:7878");
    println!("anyhow {{}}  : {wrapped}");
    println!("anyhow {{:#}}: {wrapped:#}");

    let io = FrameError::from(std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connection reset by peer"));
    println!("io      : {io}  <-  {}", io.source().unwrap());
}
