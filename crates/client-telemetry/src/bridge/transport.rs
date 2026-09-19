//! Framing + token handshake for the client bridge's inbound TCP
//! channel.
//!
//! Wire shape is deliberately identical to the Atrea editor bridge
//! (`docs/architecture/atrea-editor-bridge.md` §3.2) so the two can
//! later share a framing crate, but the code is self-contained here
//! per the live-research-lab ADR:
//!
//! - **4-byte little-endian length prefix**, then a UTF-8 JSON body.
//!   Framed (not newline-delimited) so payloads may contain newlines
//!   safely.
//! - The **first framed message** must be `{"token":"<64-hex>"}`.
//!   The token is compared in constant time; a wrong or absent token
//!   closes the connection. Everything after is JSON-RPC 2.0.
//!
//! These helpers take `std::io` traits so they run on the DLL's
//! blocking IO thread (`std::net::TcpStream`) and are unit-testable
//! against a `Cursor<Vec<u8>>` with no socket.

use std::io::{self, Read, Write};

use serde::Deserialize;
use thiserror::Error;

/// Hard cap on a single frame body. A hostile or buggy peer must not
/// be able to make us allocate an arbitrary buffer off a 4-byte
/// length it controls. 1 MiB is generous for a Lua chunk or a hex
/// memory dump and still bounds the worst case.
pub const MAX_FRAME_LEN: u32 = 1 << 20;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("frame length {0} exceeds max {MAX_FRAME_LEN}")]
    FrameTooLarge(u32),
}

/// Write one framed message: `u32` little-endian length, then `body`.
pub fn write_frame<W: Write>(w: &mut W, body: &[u8]) -> Result<(), TransportError> {
    let len = body.len();
    if len as u64 > MAX_FRAME_LEN as u64 {
        return Err(TransportError::FrameTooLarge(len as u32));
    }
    // `len` fits in u32 by the guard above.
    w.write_all(&(len as u32).to_le_bytes())?;
    w.write_all(body)?;
    w.flush()?;
    Ok(())
}

/// Read one framed message. Returns the body bytes. A length beyond
/// [`MAX_FRAME_LEN`] is rejected before any allocation. EOF while
/// reading the prefix or body surfaces as an `Io` error (the caller
/// treats a clean EOF on the prefix as "client disconnected").
pub fn read_frame<R: Read>(r: &mut R) -> Result<Vec<u8>, TransportError> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_FRAME_LEN {
        return Err(TransportError::FrameTooLarge(len));
    }
    let mut body = vec![0u8; len as usize];
    r.read_exact(&mut body)?;
    Ok(body)
}

/// The first-message auth frame: `{"token":"<hex>"}`.
#[derive(Debug, Deserialize)]
struct AuthFrame {
    token: String,
}

/// True iff `frame_body` is a well-formed auth frame whose token
/// constant-time-matches `expected`. A parse failure, a missing
/// `token` field, or a mismatch all return `false` — the caller
/// closes the connection in every case, so "wrong token" and
/// "absent token" are one code path.
pub fn verify_token_frame(frame_body: &[u8], expected: &str) -> bool {
    match serde_json::from_slice::<AuthFrame>(frame_body) {
        Ok(auth) => ct_eq(auth.token.as_bytes(), expected.as_bytes()),
        Err(_) => false,
    }
}

/// Serialize the auth frame body a client sends first.
pub fn auth_frame_body(token: &str) -> Vec<u8> {
    // Small, fixed shape — never fails to serialize.
    serde_json::to_vec(&serde_json::json!({ "token": token })).expect("auth frame serializes")
}

/// Constant-time byte-slice equality. Compares every byte regardless
/// of where the first mismatch is, so a network peer cannot time the
/// token prefix. Length mismatch short-circuits (length is not
/// secret) but still folds a difference into the accumulator.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Framing round-trip: what `write_frame` emits, `read_frame`
    /// reads back byte-for-byte, including a body with an embedded
    /// newline (the reason we frame instead of line-delimiting).
    #[test]
    fn frame_round_trip() {
        let body = br#"{"jsonrpc":"2.0",
"id":1,"method":"ping"}"#;
        let mut buf = Vec::new();
        write_frame(&mut buf, body).unwrap();

        // First 4 bytes are the LE length prefix.
        assert_eq!(&buf[..4], &(body.len() as u32).to_le_bytes());

        let mut cur = Cursor::new(buf);
        let got = read_frame(&mut cur).unwrap();
        assert_eq!(got, body);
    }

    /// Two frames back to back read out in order — the length prefix
    /// delimits them with no separator.
    #[test]
    fn two_frames_read_in_order() {
        let mut buf = Vec::new();
        write_frame(&mut buf, b"first").unwrap();
        write_frame(&mut buf, b"second").unwrap();
        let mut cur = Cursor::new(buf);
        assert_eq!(read_frame(&mut cur).unwrap(), b"first");
        assert_eq!(read_frame(&mut cur).unwrap(), b"second");
    }

    /// A declared length over the cap is rejected without allocating
    /// (we build the prefix by hand so no huge body is written).
    #[test]
    fn oversized_frame_length_rejected() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(MAX_FRAME_LEN + 1).to_le_bytes());
        let mut cur = Cursor::new(buf);
        match read_frame(&mut cur) {
            Err(TransportError::FrameTooLarge(n)) => assert_eq!(n, MAX_FRAME_LEN + 1),
            other => panic!("expected FrameTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn writing_oversized_body_rejected() {
        let mut sink = Vec::new();
        let big = vec![0u8; (MAX_FRAME_LEN + 1) as usize];
        assert!(matches!(
            write_frame(&mut sink, &big),
            Err(TransportError::FrameTooLarge(_))
        ));
    }

    #[test]
    fn correct_token_frame_accepted() {
        let expected = "a".repeat(64);
        let body = auth_frame_body(&expected);
        assert!(verify_token_frame(&body, &expected));
    }

    /// Wrong token, absent `token` field, and non-JSON garbage all
    /// fail verification — the single "close the connection" path.
    #[test]
    fn bad_token_frames_rejected() {
        let expected = "a".repeat(64);
        assert!(!verify_token_frame(&auth_frame_body("b"), &expected));
        assert!(!verify_token_frame(br#"{"nottoken":"a"}"#, &expected));
        assert!(!verify_token_frame(b"not json at all", &expected));
        assert!(!verify_token_frame(b"", &expected));
    }

    #[test]
    fn ct_eq_matches_std_eq() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"ab"));
        assert!(ct_eq(b"", b""));
    }
}
