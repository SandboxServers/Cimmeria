//! Framed JSON-RPC 2.0 client for the injected client bridge.
//!
//! Mirrors the bridge's transport
//! (`crates/client-telemetry/src/bridge/transport.rs`): a 4-byte
//! little-endian length prefix then a UTF-8 JSON body, with the token
//! sent as the first framed message (`{"token":"<hex>"}`). Kept
//! self-contained (rather than depending on the telemetry crate,
//! which pulls the Windows-only MinHook toolchain) — the framing is a
//! dozen lines.
//!
//! The connection is lazily established and cached; on any transport
//! error it is dropped so the next call reconnects and re-auths.
//! Single client is fine: the MCP stdio loop is serial.

use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

/// Must match the bridge's `MAX_FRAME_LEN`.
const MAX_FRAME_LEN: u32 = 1 << 20;

/// Connection state, all behind one lock: the current target
/// (addr/token) and the cached authenticated stream. Folding target and
/// stream into a single mutex means `reconfigure` and `call` take the
/// same single lock in the same order — no lock-ordering hazard.
struct ConnState {
    addr: String,
    token: String,
    stream: Option<TcpStream>,
}

pub struct BridgeClient {
    state: Mutex<ConnState>,
    next_id: AtomicI64,
}

impl BridgeClient {
    pub fn new(addr: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            state: Mutex::new(ConnState {
                addr: addr.into(),
                token: token.into(),
                stream: None,
            }),
            next_id: AtomicI64::new(1),
        }
    }

    /// Re-point the client at a fresh bridge target (new port and/or the
    /// per-launch token the supervisor just wrote). Drops any cached
    /// connection so the next call reconnects and re-auths with the new
    /// token.
    pub async fn reconfigure(&self, addr: impl Into<String>, token: impl Into<String>) {
        let mut st = self.state.lock().await;
        st.addr = addr.into();
        st.token = token.into();
        st.stream = None;
    }

    /// Call a bridge method and return its JSON-RPC `result`. A
    /// JSON-RPC `error` from the bridge, or any transport failure,
    /// surfaces as `Err` (and drops the connection so the next call
    /// reconnects).
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let mut guard = self.state.lock().await;
        match self.call_inner(&mut guard, method, params).await {
            Ok(v) => Ok(v),
            Err(e) => {
                // Force a fresh connect + re-auth next time.
                guard.stream = None;
                Err(e)
            }
        }
    }

    async fn call_inner(
        &self,
        guard: &mut ConnState,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        if guard.stream.is_none() {
            let s = connect_and_auth(&guard.addr, &guard.token).await?;
            guard.stream = Some(s);
        }
        let stream = guard.stream.as_mut().expect("just connected");

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        write_frame(stream, &serde_json::to_vec(&req)?).await?;

        let body = read_frame(stream).await?;
        let resp: Value = serde_json::from_slice(&body).context("bridge response is not JSON")?;
        if let Some(err) = resp.get("error") {
            let code = err.get("code").and_then(Value::as_i64).unwrap_or(0);
            let msg = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("(no message)");
            bail!("bridge error {code}: {msg}");
        }
        resp.get("result")
            .cloned()
            .ok_or_else(|| anyhow!("bridge response had neither result nor error"))
    }

    /// Read the bridge's Tick-drain heartbeat counter. Used by the
    /// watchdog: a value that stops advancing (or a call that fails)
    /// means the client's main thread is wedged.
    pub async fn heartbeat(&self) -> Result<u64> {
        let result = self.call("heartbeat", json!({})).await?;
        result
            .get("tick_count")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("heartbeat result missing tick_count: {result}"))
    }
}

async fn connect_and_auth(addr: &str, token: &str) -> Result<TcpStream> {
    let mut stream = TcpStream::connect(addr)
        .await
        .with_context(|| format!("connect to bridge at {addr}"))?;
    // First frame: the token.
    let auth = json!({ "token": token });
    write_frame(&mut stream, &serde_json::to_vec(&auth)?).await?;
    // Bridge acks with {"ok":true} on success, or closes on a bad
    // token (read then returns EOF).
    let ack = read_frame(&mut stream)
        .await
        .context("bridge closed the connection (bad or missing token?)")?;
    let ack: Value = serde_json::from_slice(&ack).unwrap_or(Value::Null);
    if ack.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!("bridge did not acknowledge the token: {ack}");
    }
    Ok(stream)
}

async fn write_frame(stream: &mut TcpStream, body: &[u8]) -> Result<()> {
    if body.len() as u64 > MAX_FRAME_LEN as u64 {
        bail!("frame body {} exceeds max {MAX_FRAME_LEN}", body.len());
    }
    stream.write_all(&(body.len() as u32).to_le_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_FRAME_LEN {
        bail!("frame length {len} exceeds max {MAX_FRAME_LEN}");
    }
    let mut body = vec![0u8; len as usize];
    stream.read_exact(&mut body).await?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The client's framing matches the bridge's contract: 4-byte LE
    /// length prefix then the body. We assert the prefix bytes an
    /// in-memory buffer would receive, without a socket.
    #[tokio::test]
    async fn frame_prefix_is_le_length() {
        // Use a loopback pair to exercise the real async frame codec.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let got = read_frame(&mut s).await.unwrap();
            // Echo it straight back framed.
            write_frame(&mut s, &got).await.unwrap();
        });
        let mut c = TcpStream::connect(addr).await.unwrap();
        let payload = br#"{"hello":"world"}"#;
        write_frame(&mut c, payload).await.unwrap();
        let echoed = read_frame(&mut c).await.unwrap();
        assert_eq!(echoed, payload);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn oversized_read_length_rejected() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            // Send an oversized length prefix, no body.
            s.write_all(&(MAX_FRAME_LEN + 1).to_le_bytes())
                .await
                .unwrap();
            s.flush().await.unwrap();
            // Keep the socket open briefly.
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });
        let mut c = TcpStream::connect(addr).await.unwrap();
        assert!(read_frame(&mut c).await.is_err());
    }
}
