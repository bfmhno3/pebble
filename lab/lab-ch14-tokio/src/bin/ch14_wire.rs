use pebble_net::{ServerConfig, serve_listener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    let server = tokio::spawn(serve_listener(listener, ServerConfig::default(), async move {
        let _ = stopped.await;
    }));

    let program = "1 + 2 * 3;";
    let mut request = (program.len() as u32).to_be_bytes().to_vec();
    request.extend_from_slice(program.as_bytes());
    println!("request  : {}", hex(&request));

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(&request).await.unwrap();

    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await.unwrap();
    let mut payload = vec![0u8; u32::from_be_bytes(header) as usize];
    stream.read_exact(&mut payload).await.unwrap();
    println!("response : {} {}", hex(&header), hex(&payload));
    println!("as text  : {:?}", String::from_utf8(payload).unwrap());

    let _ = stop.send(());
    server.await.unwrap().unwrap();
}
