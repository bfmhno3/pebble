use std::io;

use tokio::io::{AsyncRead, AsyncReadExt};

async fn read_frame<R>(reader: &mut R, max_frame: usize) -> io::Result<Option<Vec<u8>>>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0u8; 4];
    match reader.read_exact(&mut header).await {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }

    let length = u32::from_be_bytes(header) as usize;
    if length > max_frame {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("frame length {length} exceeds maximum frame size {max_frame}"),
        ));
    }

    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

#[tokio::main]
async fn main() {
    let mut too_big: &[u8] = &1024u32.to_be_bytes();
    let error = read_frame(&mut too_big, 16).await.unwrap_err();
    println!("kind    : {:?}", error.kind());
    println!("message : {error}");

    // A truncated payload is also an io::Error, with a different kind.
    let mut truncated: &[u8] = &[0, 0, 0, 8, b'1', b'+'];
    let error = read_frame(&mut truncated, 16).await.unwrap_err();
    println!("kind    : {:?}", error.kind());
    println!("message : {error}");
}
