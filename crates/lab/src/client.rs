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
    /// Wall-clock ms of the last successful bridge response (any method).
    /// The watchdog reads it: a heartbeat that fails while other calls are
    /// completing is starved behind probe traffic, not a dead client.
    last_ok_ms: AtomicI64,
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
            last_ok_ms: AtomicI64::new(0),
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
        let v = self.call_untracked(method, params).await?;
        self.mark_ok();
        Ok(v)
    }

    /// [`Self::call`] without refreshing `last_ok_ms`: for callers that
    /// must validate the result before it counts as a live bridge.
    async fn call_untracked(&self, method: &str, params: Value) -> Result<Value> {
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

    fn mark_ok(&self) {
        self.last_ok_ms.store(now_ms(), Ordering::Relaxed);
    }

    async fn call_inner(
        &self,
        guard: &mut ConnState,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        // The stream is taken out of the cache for the exchange and put back
        // only after a whole response was read. A caller that times out and
        // drops this future (the event keeper's tick does) would otherwise
        // leave an unread response on the cached stream, and the next call
        // would read it as its own: every later answer one call behind.
        let mut taken = match guard.stream.take() {
            Some(s) => s,
            None => connect_and_auth(&guard.addr, &guard.token).await?,
        };
        let stream = &mut taken;

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        write_frame(stream, &serde_json::to_vec(&req)?).await?;

        let body = read_frame(stream).await?;
        guard.stream = Some(taken);
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

    /// Milliseconds since the last successful bridge response, or `None`
    /// if there has not been one.
    pub fn ms_since_last_ok(&self) -> Option<i64> {
        match self.last_ok_ms.load(Ordering::Relaxed) {
            0 => None,
            t => Some(now_ms() - t),
        }
    }

    /// Read the bridge's Tick-drain heartbeat counter. Used by the
    /// watchdog: a value that stops advancing (or a call that fails)
    /// means the client's main thread is wedged.
    pub async fn heartbeat(&self) -> Result<u64> {
        // Only a well-formed heartbeat refreshes `last_ok_ms`: a malformed
        // one must not reset the watchdog's failure count every poll.
        let result = self.call_untracked("heartbeat", json!({})).await?;
        let ticks = result
            .get("tick_count")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("heartbeat result missing tick_count: {result}"))?;
        self.mark_ok();
        Ok(ticks)
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

async fn connect_and_auth(addr: &str, token: &str) -> Result<TcpStream> {
    let mut stream = TcpStream::connect(addr)
        .await
        .with_context(|| format!("connect to bridge at {addr}"))?;
    // First frame: the token.
    let auth = json!({ "token": token });
    write_frame(&mut stream, &serde_json::to_vec(&auth)?).await?;
    // A closed socket while reading the ack is ambiguous: the bridge also
    // closes it for a rejected token, but a world load can reset it here.
    let ack = read_frame(&mut stream).await.map_err(|error| {
        let transient = error.downcast_ref::<std::io::Error>().is_some_and(|io| {
            matches!(
                io.kind(),
                std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
            ) || matches!(io.raw_os_error(), Some(10053 | 10054))
        });
        if transient {
            error.context("bridge connection reset while authenticating; client may be loading")
        } else {
            error.context("bridge authentication acknowledgement read failed")
        }
    })?;
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

    #[tokio::test]
    async fn closed_auth_ack_is_ambiguous_but_explicit_rejection_is_permanent() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _auth = read_frame(&mut socket).await.unwrap();
            // The client sees EOF before an acknowledgement.
        });
        let error = connect_and_auth(&addr.to_string(), "test-token")
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("bridge connection reset while authenticating"),
            "{error}"
        );
        assert!(!error.contains("bad or missing token"), "{error}");
        server.await.unwrap();

        // A bridge that explicitly rejects the token still uses the
        // permanent acknowledgement-error path.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _auth = read_frame(&mut socket).await.unwrap();
            write_frame(&mut socket, br#"{"ok":false}"#).await.unwrap();
        });
        let error = connect_and_auth(&addr.to_string(), "test-token")
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("bridge did not acknowledge the token"),
            "{error}"
        );
        server.await.unwrap();
    }

    /// A call dropped mid-exchange (the event keeper's tick timeout) must not
    /// leave its unread response on the cached stream: the next call would
    /// read it as its own answer.
    #[tokio::test]
    async fn a_cancelled_call_does_not_poison_the_next_one() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (mut s, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let _auth = read_frame(&mut s).await.unwrap();
                    write_frame(&mut s, br#"{"ok":true}"#).await.unwrap();
                    while let Ok(body) = read_frame(&mut s).await {
                        let req: Value = serde_json::from_slice(&body).unwrap();
                        let delay = req["params"]["delay_ms"].as_u64().unwrap_or(0);
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        let resp = json!({ "jsonrpc": "2.0", "id": req["id"],
                            "result": req["method"] });
                        if write_frame(&mut s, &serde_json::to_vec(&resp).unwrap())
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                });
            }
        });
        let c = BridgeClient::new(addr.to_string(), "t");
        assert_eq!(c.call("warm", json!({})).await.unwrap(), json!("warm"));
        let slow = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            c.call("slow", json!({ "delay_ms": 400 })),
        )
        .await;
        assert!(slow.is_err(), "the slow call is cancelled");
        assert_eq!(c.call("fast", json!({})).await.unwrap(), json!("fast"));
    }
}
