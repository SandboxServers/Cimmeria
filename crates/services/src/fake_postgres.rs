//! A stand-in PostgreSQL server for tests of the startup database checks.
//!
//! It answers the startup handshake with one `ErrorResponse` carrying the
//! given SQLSTATE and message, then closes, the way a real server rejects a
//! bad password (`28P01`) or an unknown database (`3D000`). No database is
//! needed, so the tests run in the plain `cargo test` tier.

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Bind `127.0.0.1:0`, reject every login with `sqlstate` / `message`, and
/// return the port.
pub(crate) async fn spawn_rejecting_postgres(sqlstate: &'static str, message: &'static str) -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                // Startup packets: int32 length (inclusive), int32 code. An
                // SSLRequest (80877103) gets 'N', then the real startup
                // message follows.
                loop {
                    let mut len = [0u8; 4];
                    if sock.read_exact(&mut len).await.is_err() {
                        return;
                    }
                    let len = u32::from_be_bytes(len) as usize;
                    let mut body = vec![0u8; len.saturating_sub(4)];
                    if sock.read_exact(&mut body).await.is_err() {
                        return;
                    }
                    if body.len() >= 4 && body[..4] == 80_877_103u32.to_be_bytes() {
                        let _ = sock.write_all(b"N").await;
                        continue;
                    }
                    break;
                }
                let mut fields = Vec::new();
                for (code, value) in [
                    (b'S', "FATAL"),
                    (b'V', "FATAL"),
                    (b'C', sqlstate),
                    (b'M', message),
                ] {
                    fields.push(code);
                    fields.extend_from_slice(value.as_bytes());
                    fields.push(0);
                }
                fields.push(0);
                let mut msg = vec![b'E'];
                msg.extend_from_slice(&((fields.len() + 4) as u32).to_be_bytes());
                msg.extend_from_slice(&fields);
                let _ = sock.write_all(&msg).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    port
}
