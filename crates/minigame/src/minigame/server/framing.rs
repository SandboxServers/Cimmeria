//! Null-terminated message framing over the raw TCP socket.
//!
//! SmartFoxServer 1.x delimits every XML message with a single `0x00`
//! byte — there is no length prefix, so the reader has to scan for the
//! terminator and carry the remainder of a coalesced read forward.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Maximum size of a single framed message.
///
/// Matches C++ `MinigameConnection::MaxMessageLength`
/// (`deprecated/cpp/src/baseapp/minigame_connection.hpp`). A read that
/// fills the buffer without finding a terminator is treated as a protocol
/// violation and drops the connection.
pub(super) const MAX_MESSAGE_LEN: usize = 0x1000;

/// Read a null-terminated message from the stream.
///
/// `buf_len` is how many bytes of `buf` hold unconsumed data across calls:
/// one `read` can deliver several messages, so the tail is shifted down
/// rather than discarded. Returns `None` on EOF, read error, or an
/// over-long message.
pub(super) async fn read_null_terminated(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
) -> Option<String> {
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
            return Some(msg);
        }

        // Need more data
        if *buf_len >= buf.len() {
            tracing::warn!("Minigame message too long (>{} bytes)", buf.len());
            return None;
        }

        match stream.read(&mut buf[*buf_len..]).await {
            Ok(0) => return None, // EOF
            Ok(n) => *buf_len += n,
            Err(e) => {
                tracing::debug!(error = %e, "Minigame read error");
                return None;
            }
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
