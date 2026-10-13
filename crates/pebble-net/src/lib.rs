#![forbid(unsafe_code)]
//! An asynchronous remote evaluator for Pebble, built on Tokio.
//!
//! The wire protocol is deliberately tiny: every frame is a 4-byte
//! big-endian `u32` length prefix followed by exactly that many UTF-8 bytes.
//! A client sends a Pebble program as a request frame; the server replies with
//! a response frame containing the rendered result or diagnostics.
//!
//! ```text
//! client                                            server
//!   |  <len:u32><program bytes>  ------------------>  read_frame
//!   |                                                 evaluate (spawn_blocking)
//!   |  <------- <len:u32><rendered text>             write_frame
//! ```
//!
//! Evaluation itself is synchronous — the tree-walking interpreter owns
//! `Rc`-based environments and is therefore not `Send` — so it runs inside
//! [`tokio::task::spawn_blocking`], and the resulting `String` is written back
//! only after the non-`Send` interpreter has been dropped. Each connection is
//! driven by its own spawned task, sharing a [`Stats`] counter behind a
//! [`tokio::sync::Mutex`].

use std::future::Future;
use std::io;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use pebble_diag::Diagnostic;
use pebble_interp::Interpreter;
use pebble_parser::parse;
use pebble_span::SourceMap;
use pebble_value::{Output, RuntimeError};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::Mutex;

/// The default maximum frame size: 1 MiB.
pub const DEFAULT_MAX_FRAME: usize = 1024 * 1024;

/// The default per-request timeout.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Tunables for the evaluator server.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Largest request (and response) frame accepted, in bytes. A peer that
    /// announces a larger frame is disconnected.
    pub max_frame: usize,
    /// How long a single request may spend evaluating before the server gives
    /// up and returns a timeout message.
    pub request_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_frame: DEFAULT_MAX_FRAME,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }
}

/// Cumulative counters shared by every connection.
#[derive(Default, Debug)]
pub struct Stats {
    /// Connections accepted so far.
    pub connections: u64,
    /// Well-framed requests received so far.
    pub requests: u64,
    /// Requests that produced diagnostics, timed out, or failed to decode,
    /// plus framing errors.
    pub errors: u64,
}

// ---------------------------------------------------------------------------
// Pure evaluation
// ---------------------------------------------------------------------------

/// The evaluator's output plus whether it describes an error.
struct Evaluation {
    /// The text to send back to the client.
    text: String,
    /// Whether `text` represents a parse or runtime error.
    is_error: bool,
}

/// Collects interpreter output into memory so it can be read after the
/// interpreter (which takes ownership of its `Box<dyn Output>`) is dropped.
#[derive(Clone, Default)]
struct SharedOutput {
    lines: Arc<StdMutex<Vec<String>>>,
}

impl SharedOutput {
    /// A fresh, empty sink.
    fn new() -> Self {
        Self::default()
    }

    /// The captured output as a newline-joined string.
    fn text(&self) -> String {
        let lines = self
            .lines
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        lines.join("\n")
    }
}

impl Output for SharedOutput {
    fn write_line(&mut self, line: &str) {
        let mut lines = self
            .lines
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        lines.push(line.to_string());
    }
}

/// Parses and interprets `program`, returning the rendered text plus whether
/// it is an error. This never touches the network and never panics on bad
/// input.
fn evaluate(program: &str) -> Evaluation {
    let mut sources = SourceMap::new();
    let source = sources.add("<remote>", program);
    let parsed = parse(source, program);

    if !parsed.diagnostics.is_empty() {
        return Evaluation {
            text: render_diagnostics(&parsed.diagnostics, &sources),
            is_error: true,
        };
    }

    let output = SharedOutput::new();
    let mut interpreter = Interpreter::with_output(Box::new(output.clone()));
    let result = interpreter.interpret(&parsed.program);
    // Drop the interpreter before awaiting anything: it is not `Send`.
    drop(interpreter);

    let printed = output.text();
    let mut parts: Vec<String> = Vec::new();
    if !printed.is_empty() {
        parts.push(printed);
    }

    let is_error = match result {
        Ok(value) => {
            parts.push(value.to_string());
            false
        }
        Err(error) => {
            parts.push(render_runtime_error(&error, &sources));
            true
        }
    };

    Evaluation {
        text: parts.join("\n"),
        is_error,
    }
}

/// Parses and interprets `program` and returns either the `Display` rendering
/// of its result or the rendered diagnostics text. Captured `print` output is
/// prepended when present. This is pure: no networking, no shared state.
#[must_use]
pub fn eval_to_string(program: &str) -> String {
    evaluate(program).text
}

/// Renders parse/lex diagnostics against `sources`.
fn render_diagnostics(diagnostics: &[Diagnostic], sources: &SourceMap) -> String {
    let rendered: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.render(sources))
        .collect();
    rendered.join("\n")
}

/// Renders a runtime error and its stack trace against `sources`.
fn render_runtime_error(error: &RuntimeError, sources: &SourceMap) -> String {
    let mut text = error.diagnostic.render(sources);
    if !error.trace.is_empty() {
        text.push_str("stack trace:\n");
        for frame in &error.trace {
            text.push_str(&format!("  at {} ({:?})\n", frame.function, frame.span));
        }
    }
    text
}

// ---------------------------------------------------------------------------
// Framing
// ---------------------------------------------------------------------------

/// Why a frame could not be read or written.
#[derive(Debug)]
#[non_exhaustive]
pub enum FrameError {
    /// The underlying connection failed.
    Io(io::Error),
    /// A frame announced or carried more bytes than the configured maximum.
    TooLarge {
        /// The offending length in bytes.
        length: usize,
        /// The largest length that is allowed.
        max: usize,
    },
    /// A frame payload that should be text was not valid UTF-8.
    InvalidUtf8(std::string::FromUtf8Error),
    /// The peer closed the connection before sending a response.
    Closed,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(_) => f.write_str("i/o error on the connection"),
            Self::TooLarge { length, max } => {
                write!(f, "frame length {length} exceeds maximum frame size {max}")
            }
            Self::InvalidUtf8(_) => f.write_str("frame payload is not valid UTF-8"),
            Self::Closed => f.write_str("peer closed the connection without a response"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidUtf8(error) => Some(error),
            Self::TooLarge { .. } | Self::Closed => None,
        }
    }
}

impl From<io::Error> for FrameError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<std::string::FromUtf8Error> for FrameError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Self::InvalidUtf8(error)
    }
}

/// Reads one length-prefixed frame. Returns `Ok(None)` on a clean end of
/// stream before any header byte was read.
async fn read_frame<R>(reader: &mut R, max_frame: usize) -> Result<Option<Vec<u8>>, FrameError>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0u8; 4];
    match reader.read_exact(&mut header).await {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.into()),
    }

    let length = u32::from_be_bytes(header) as usize;
    if length > max_frame {
        return Err(FrameError::TooLarge {
            length,
            max: max_frame,
        });
    }

    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

/// Writes one length-prefixed frame.
async fn write_frame<W>(writer: &mut W, payload: &[u8], max_frame: usize) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
{
    if payload.len() > max_frame {
        return Err(FrameError::TooLarge {
            length: payload.len(),
            max: max_frame,
        });
    }
    let length = u32::try_from(payload.len()).map_err(|_| FrameError::TooLarge {
        length: payload.len(),
        max: u32::MAX as usize,
    })?;
    writer.write_all(&length.to_be_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

/// Serves remote Pebble evaluations on `addr` until the process receives
/// `Ctrl-C`.
///
/// # Errors
///
/// Returns the binding or accept error if the listener cannot be established.
pub async fn serve<A: ToSocketAddrs>(addr: A, config: ServerConfig) -> io::Result<()> {
    let shutdown = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            eprintln!("pebble-net: could not listen for Ctrl-C: {error}");
        }
    };
    serve_with_shutdown(addr, config, shutdown).await
}

/// Binds `addr` and serves until `shutdown` resolves.
///
/// # Errors
///
/// Returns the binding or accept error if the listener cannot be established.
pub async fn serve_with_shutdown<A: ToSocketAddrs>(
    addr: A,
    config: ServerConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    serve_listener(listener, config, shutdown).await
}

/// Serves on an already-bound listener until `shutdown` resolves.
///
/// Each accepted connection is handled by a spawned task, so a slow or idle
/// client never blocks the accept loop.
///
/// # Errors
///
/// Returns an error if accepting a connection fails at the OS level.
pub async fn serve_listener(
    listener: TcpListener,
    config: ServerConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    let stats = Arc::new(Mutex::new(Stats::default()));
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            () = &mut shutdown => return Ok(()),
            accepted = listener.accept() => {
                let (stream, _peer) = accepted?;
                let config = config.clone();
                let stats = Arc::clone(&stats);
                tokio::spawn(async move {
                    let _ = handle_connection(stream, config, stats).await;
                });
            }
        }
    }
}

/// Handles a single client connection: reads request frames until the peer
/// disconnects, evaluating each one with a per-request timeout.
///
/// # Errors
///
/// Returns a [`FrameError`] when framing fails (including an oversized frame),
/// the payload is not UTF-8, or the connection breaks mid-frame.
pub async fn handle_connection(
    stream: TcpStream,
    config: ServerConfig,
    stats: Arc<Mutex<Stats>>,
) -> Result<(), FrameError> {
    stats.lock().await.connections += 1;
    let (mut reader, mut writer) = stream.into_split();

    loop {
        let payload = match read_frame(&mut reader, config.max_frame).await {
            Ok(Some(payload)) => payload,
            Ok(None) => return Ok(()),
            Err(error) => {
                stats.lock().await.errors += 1;
                return Err(error);
            }
        };

        let program = match String::from_utf8(payload) {
            Ok(program) => program,
            Err(error) => {
                stats.lock().await.errors += 1;
                return Err(error.into());
            }
        };

        stats.lock().await.requests += 1;

        let evaluated = tokio::task::spawn_blocking(move || evaluate(&program));
        let response = match tokio::time::timeout(config.request_timeout, evaluated).await {
            Ok(Ok(evaluation)) => {
                if evaluation.is_error {
                    stats.lock().await.errors += 1;
                }
                evaluation.text
            }
            Ok(Err(join_error)) => {
                stats.lock().await.errors += 1;
                format!("error: evaluator task failed: {join_error}")
            }
            Err(_elapsed) => {
                stats.lock().await.errors += 1;
                "error: request timed out".to_string()
            }
        };

        write_frame(&mut writer, response.as_bytes(), config.max_frame).await?;
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Connects to `addr`, sends `program`, and returns the server's rendered
/// response.
///
/// # Errors
///
/// Returns an error if the connection fails, the response is malformed, or the
/// server disconnects without replying ([`FrameError::Closed`]).
pub async fn run_client(addr: impl ToSocketAddrs, program: &str) -> Result<String, FrameError> {
    let mut stream = TcpStream::connect(addr).await?;
    write_frame(&mut stream, program.as_bytes(), DEFAULT_MAX_FRAME).await?;

    let payload = read_frame(&mut stream, DEFAULT_MAX_FRAME)
        .await?
        .ok_or(FrameError::Closed)?;

    Ok(String::from_utf8(payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::oneshot;

    #[test]
    fn eval_to_string_returns_the_value() {
        assert_eq!(eval_to_string("1 + 2 * 3;"), "7");
    }

    #[test]
    fn eval_to_string_reports_parse_errors() {
        assert!(eval_to_string("1 + ").to_lowercase().contains("error"));
    }

    #[test]
    fn eval_to_string_includes_print_output() {
        assert!(eval_to_string("print(\"hi\");").contains("hi"));
    }

    #[test]
    fn eval_to_string_reports_runtime_errors() {
        let text = eval_to_string("1 / 0;").to_lowercase();
        assert!(text.contains("error"), "unexpected text: {text}");
    }

    /// Binds an ephemeral port and returns the listener plus its address.
    async fn bound_listener() -> (TcpListener, std::net::SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        (listener, addr)
    }

    #[tokio::test]
    async fn round_trip_over_tcp() {
        let (listener, addr) = bound_listener().await;
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server = tokio::spawn(serve_listener(
            listener,
            ServerConfig::default(),
            async move {
                let _ = shutdown_rx.await;
            },
        ));

        let ok = run_client(addr, "1 + 2 * 3;").await.expect("client ok");
        assert_eq!(ok, "7");

        let bad = run_client(addr, "1 + ").await.expect("client bad");
        assert!(bad.to_lowercase().contains("error"), "unexpected: {bad}");

        let _ = shutdown_tx.send(());
        server.await.expect("join").expect("serve");
    }

    #[tokio::test]
    async fn concurrent_clients_both_succeed() {
        let (listener, addr) = bound_listener().await;
        let stats = Arc::new(Mutex::new(Stats::default()));

        let accept = {
            let stats = Arc::clone(&stats);
            tokio::spawn(async move {
                for _ in 0..2 {
                    let (stream, _peer) = listener.accept().await.expect("accept");
                    let stats = Arc::clone(&stats);
                    tokio::spawn(handle_connection(stream, ServerConfig::default(), stats));
                }
            })
        };

        let (first, second) = tokio::join!(run_client(addr, "1 + 1;"), run_client(addr, "2 + 2;"),);
        assert_eq!(first.expect("first"), "2");
        assert_eq!(second.expect("second"), "4");

        accept.await.expect("join accept");
        assert_eq!(stats.lock().await.requests, 2);
    }

    #[tokio::test]
    async fn oversized_frame_is_rejected() {
        let (listener, addr) = bound_listener().await;
        let config = ServerConfig {
            max_frame: 16,
            ..ServerConfig::default()
        };
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server = tokio::spawn(serve_listener(listener, config, async move {
            let _ = shutdown_rx.await;
        }));

        let mut stream = TcpStream::connect(addr).await.expect("connect");
        let announced = 1024u32;
        stream
            .write_all(&announced.to_be_bytes())
            .await
            .expect("write header");

        // The server must close the connection rather than reading the frame.
        let mut buffer = [0u8; 4];
        match stream.read(&mut buffer).await {
            Ok(0) => {}
            Err(_) => {}
            Ok(n) => panic!("expected disconnect, read {n} bytes"),
        }

        let _ = shutdown_tx.send(());
        server.await.expect("join").expect("serve");
    }

    #[tokio::test]
    async fn oversized_header_is_a_typed_error() {
        let mut reader: &[u8] = &1024u32.to_be_bytes();
        let error = read_frame(&mut reader, 16).await.unwrap_err();
        assert!(
            matches!(
                error,
                FrameError::TooLarge {
                    length: 1024,
                    max: 16
                }
            ),
            "got {error:?}"
        );
        assert_eq!(
            error.to_string(),
            "frame length 1024 exceeds maximum frame size 16"
        );
    }

    #[test]
    fn invalid_utf8_keeps_its_source() {
        let error = FrameError::from(String::from_utf8(vec![0xff]).unwrap_err());
        assert_eq!(error.to_string(), "frame payload is not valid UTF-8");
        assert!(std::error::Error::source(&error).is_some());
    }
}
