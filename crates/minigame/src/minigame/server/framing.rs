//! Null-terminated message framing over the raw TCP socket.
//!
//! SmartFoxServer 1.x delimits every XML message with a single `0x00`
//! byte — there is no length prefix, so the reader has to scan for the
//! terminator and carry the remainder of a coalesced read forward.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Maximum size of a single framed message, and the size of the one read
/// buffer each connection owns.
///
/// Matches C++ `MinigameConnection::MaxMessageLength`
/// (`deprecated/cpp/src/baseapp/minigame_connection.hpp`). A read that
/// fills the buffer without finding a terminator is treated as a protocol
/// violation and drops the connection. The SWFs' largest inbound frame is
/// the login (a game name, an entity id and an eight-character ticket), a
/// few hundred bytes at most.
pub(super) const MAX_MESSAGE_LEN: usize = 0x1000;

/// Why [`read_null_terminated`] produced no message.
///
/// The reader does not log: whether an over-long frame is a scanner or a
/// misbehaving SWF depends on whether the peer has logged in, which only
/// the caller knows.
#[derive(Debug)]
pub(super) enum ReadError {
    /// The peer closed the connection.
    Closed,
    /// The buffer filled without a terminator.
    TooLong,
    /// The socket read failed.
    Io(std::io::Error),
}

/// Read a null-terminated message from the stream.
///
/// `buf_len` is how many bytes of `buf` hold unconsumed data across calls:
/// one `read` can deliver several messages, so the tail is shifted down
/// rather than discarded. `buf` is never grown, so its length is the
/// message size cap.
///
/// Cancel-safe: `buf_len` only moves after a completed `read`, so dropping
/// the future inside a `select!` loses no bytes.
pub(super) async fn read_null_terminated(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
) -> Result<String, ReadError> {
    loop {
        // Check if we already have a complete message in the buffer
        if let Some(null_pos) = buf[..*buf_len].iter().position(|&b| b == 0) {
            let msg = String::from_utf8_lossy(&buf[..null_pos]).to_string();
            // Shift remaining data
            let remaining = *buf_len - null_pos - 1;
            if remaining > 0 {
                buf.copy_within(null_pos + 1..*buf_len, 0);
            }
            *buf_len = remaining;
            return Ok(msg);
        }

        // Need more data
        if *buf_len >= buf.len() {
            return Err(ReadError::TooLong);
        }

        match stream.read(&mut buf[*buf_len..]).await {
            Ok(0) => return Err(ReadError::Closed),
            Ok(n) => *buf_len += n,
            Err(e) => return Err(ReadError::Io(e)),
        }
    }
}

/// Send a null-terminated message to the stream.
pub(super) async fn send_null_terminated(stream: &mut TcpStream, msg: &str) -> Result<(), ()> {
    let mut data = msg.as_bytes().to_vec();
    data.push(0); // null terminator
    stream.write_all(&data).await.map_err(|e| {
        tracing::debug!(error = %e, "Minigame send error");
    })
}

/// A short, escaped prefix of a rejected frame for a DEBUG row, so a probe
/// can be identified from SigNoz without logging whatever a peer chose to
/// send in full.
pub(super) fn frame_sample(msg: &str) -> String {
    const SAMPLE_CHARS: usize = 48;
    msg.chars()
        .take(SAMPLE_CHARS)
        .flat_map(char::escape_debug)
        .collect()
}
