//! Live Research Lab **client bridge** — an inbound command channel
//! bolted onto the already-injected telemetry DLL, behind the
//! `lab-bridge` cargo feature.
//!
//! See `docs/architecture/live-research-lab.md` §3.3. This is phase 1:
//! transport + token + Tick-drain dispatch + `lua_eval` /
//! `module_info` / `mem_read`.
//!
//! # Double activation gate
//!
//! 1. **Compile-time**: this module only exists under
//!    `--features lab-bridge` (off by default), so any telemetry DLL
//!    handed to a normal player physically lacks it.
//! 2. **Run-time**: even when compiled in, [`maybe_start`] returns
//!    `None` unless `current-session.json` carries a `lab` block
//!    (written only by the lab supervisor). No block → no listener.
//!
//! # Threading
//!
//! An IO thread accepts a single client, checks the token, and does
//! nothing but parse/validate/queue and serialize responses. It
//! **never** touches the Lua VM or process memory. Dispatch happens
//! on the game main thread, draining the bounded queue inside the
//! `FEngineLoop::Tick` hook (see [`drain_main_thread`], wired from
//! `hooks::inline_hooks::engine_frame`). Queue overflow returns a
//! JSON-RPC error and never blocks the accept loop.

use std::io;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use serde_json::Value;

pub mod dispatch;
pub mod lua_eval;
pub mod memory;
pub mod transport;

pub use crate::session::LabConfig;
pub use dispatch::{RpcError, RpcRequest, RpcResponse};
pub use memory::ModuleInfo;
pub use transport::{TransportError, MAX_FRAME_LEN};

/// Bounded dispatch queue capacity. Small on purpose — the client is
/// synchronous (one outstanding request), so this is headroom, not a
/// buffer, and overflow is a defensive error path, not a normal
/// state.
pub const QUEUE_CAPACITY: usize = 64;

/// Per-request wait for the main thread to drain + dispatch. If the
/// main thread is wedged (modal load, crash dialog) the IO thread
/// returns an error rather than hanging the client forever.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// A parsed request plus the one-shot channel the main thread sends
/// its response back on.
struct PendingRequest {
    request: RpcRequest,
    respond: Sender<RpcResponse>,
}

/// Handle to a running bridge. Holds the consumer end of the dispatch
/// queue; the main thread drains it via [`BridgeHandle::drain`].
pub struct BridgeHandle {
    req_rx: Receiver<PendingRequest>,
    local_addr: SocketAddr,
}

impl BridgeHandle {
    /// The address the listener actually bound (useful when the
    /// config port was 0, e.g. in tests).
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Drain every queued request, dispatch it on the calling
    /// thread, and send the response back to the waiting IO thread.
    ///
    /// **Call from the main thread only** — dispatch reaches into the
    /// Lua VM and process memory.
    pub fn drain(&self) {
        while let Ok(pending) = self.req_rx.try_recv() {
            let resp = dispatch::dispatch(&pending.request);
            // Client may have disconnected mid-flight; that's fine.
            let _ = pending.respond.send(resp);
        }
    }
}

/// A lab token is a 32-byte value as 64 lowercase hex chars.
fn valid_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Start the bridge from an explicit config. Binds the listener
/// (surfacing bind errors to the caller) and spawns the IO thread.
///
/// Refuses a token that isn't 64 hex chars — the token is mandatory
/// regardless of bind address (per the ADR).
pub fn start(cfg: LabConfig) -> io::Result<BridgeHandle> {
    if !valid_token(&cfg.token) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "lab token must be 32 bytes (64 hex chars)",
        ));
    }
    let listener = TcpListener::bind((cfg.bind.as_str(), cfg.port))?;
    let local_addr = listener.local_addr()?;
    let (req_tx, req_rx) = bounded::<PendingRequest>(QUEUE_CAPACITY);
    let token = cfg.token.clone();
    std::thread::Builder::new()
        .name("cimmeria-lab-bridge-io".into())
        .spawn(move || accept_loop(listener, token, req_tx))?;
    Ok(BridgeHandle { req_rx, local_addr })
}

/// The bridge half of the double gate: returns `None` (never binds a
/// socket) unless the session carries a `lab` block.
pub fn maybe_start(session: &crate::session::DllSession) -> Option<BridgeHandle> {
    let cfg = session.lab.clone()?;
    match start(cfg) {
        Ok(h) => Some(h),
        Err(_) => None,
    }
}

/// Process-global bridge handle, set once by `boot` after a
/// successful [`maybe_start`], read by the Tick-drain hook.
static BRIDGE: OnceLock<BridgeHandle> = OnceLock::new();

/// Install the handle so [`drain_main_thread`] can find it. Returns
/// `false` if a handle was already installed.
pub fn install_handle(handle: BridgeHandle) -> bool {
    BRIDGE.set(handle).is_ok()
}

/// Main-thread drain entry point, called once per frame from the
/// `FEngineLoop::Tick` hook. No-op until a bridge is installed.
pub fn drain_main_thread() {
    if let Some(handle) = BRIDGE.get() {
        handle.drain();
    }
}

/// IO thread: accept connections, enforce single-client, hand each to
/// [`serve_client`] on its own thread so `accept` keeps running and a
/// concurrent second client is closed immediately.
fn accept_loop(listener: TcpListener, token: String, req_tx: Sender<PendingRequest>) {
    let active = Arc::new(AtomicBool::new(false));
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        // Single client: if one is already active, close the new one.
        if active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            let _ = stream.shutdown(Shutdown::Both);
            continue;
        }
        let token = token.clone();
        let req_tx = req_tx.clone();
        let active_slot = active.clone();
        let spawned = std::thread::Builder::new()
            .name("cimmeria-lab-bridge-conn".into())
            .spawn(move || {
                serve_client(stream, &token, &req_tx);
                active_slot.store(false, Ordering::SeqCst);
            });
        if spawned.is_err() {
            // Couldn't spawn the per-connection thread; release the
            // slot so the next connection isn't locked out.
            active.store(false, Ordering::SeqCst);
        }
    }
}

/// Serve one authenticated client: token frame first, then a
/// request/response loop. Never touches the VM or memory.
fn serve_client(mut stream: TcpStream, token: &str, req_tx: &Sender<PendingRequest>) {
    // First frame must be the token.
    let first = match transport::read_frame(&mut stream) {
        Ok(f) => f,
        Err(_) => return,
    };
    if !transport::verify_token_frame(&first, token) {
        let _ = stream.shutdown(Shutdown::Both);
        return;
    }
    // Ack so the client knows it's authenticated.
    if transport::write_frame(&mut stream, br#"{"ok":true}"#).is_err() {
        return;
    }
    loop {
        let body = match transport::read_frame(&mut stream) {
            Ok(b) => b,
            Err(_) => break, // disconnect or transport error
        };
        let response = handle_request_body(&body, req_tx);
        if transport::write_frame(&mut stream, &response.to_frame_body()).is_err() {
            break;
        }
    }
}

/// Parse a request body, enqueue it for the main thread, and wait for
/// the response (or return an error without ever blocking accept).
fn handle_request_body(body: &[u8], req_tx: &Sender<PendingRequest>) -> RpcResponse {
    let request: RpcRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => {
            return RpcResponse::error(Value::Null, dispatch::PARSE_ERROR, format!("parse: {e}"))
        }
    };
    let id = request.id.clone();
    match enqueue(req_tx, request) {
        Ok(resp_rx) => match resp_rx.recv_timeout(RESPONSE_TIMEOUT) {
            Ok(resp) => resp,
            Err(_) => RpcResponse::error(id, dispatch::INTERNAL_ERROR, "dispatch timeout"),
        },
        Err(resp) => resp,
    }
}

/// Try to push a request onto the bounded queue. On success returns
/// the response receiver; on overflow (or a dead consumer) returns
/// the error response to send back — never blocks.
fn enqueue(
    req_tx: &Sender<PendingRequest>,
    request: RpcRequest,
) -> Result<Receiver<RpcResponse>, RpcResponse> {
    let (resp_tx, resp_rx) = bounded::<RpcResponse>(1);
    match req_tx.try_send(PendingRequest {
        request,
        respond: resp_tx,
    }) {
        Ok(()) => Ok(resp_rx),
        Err(TrySendError::Full(p)) => Err(RpcResponse::error(
            p.request.id,
            dispatch::QUEUE_FULL,
            "bridge dispatch queue full",
        )),
        Err(TrySendError::Disconnected(p)) => Err(RpcResponse::error(
            p.request.id,
            dispatch::INTERNAL_ERROR,
            "bridge queue disconnected",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{DllSession, TelemetryBlock};

    fn a_request(id: i64) -> RpcRequest {
        RpcRequest {
            id: Value::from(id),
            method: "module_info".to_string(),
            params: Value::Null,
        }
    }

    fn a_session(lab: Option<LabConfig>) -> DllSession {
        DllSession {
            install_id: "i".into(),
            machine_id: "m".into(),
            session_id: "s".into(),
            telemetry: TelemetryBlock {
                enabled: true,
                token: "t".into(),
                upload_endpoint: "u".into(),
                expires_at_ms: 0,
                chunk_max_bytes: 0,
                flush_interval_ms: 0,
            },
            lab,
        }
    }

    fn lab_cfg(token: &str) -> LabConfig {
        LabConfig {
            bind: "127.0.0.1".into(),
            port: 0, // ephemeral
            token: token.to_string(),
        }
    }

    /// **The core guard**: no `lab` block ⇒ `maybe_start` returns
    /// `None` and binds nothing. Reverting the `?`/`None` gate so it
    /// starts unconditionally trips this.
    #[test]
    fn maybe_start_none_without_lab_block() {
        assert!(maybe_start(&a_session(None)).is_none());
    }

    #[test]
    fn maybe_start_some_with_lab_block() {
        let h = maybe_start(&a_session(Some(lab_cfg(&"e".repeat(64)))));
        assert!(h.is_some());
    }

    #[test]
    fn start_rejects_short_token() {
        assert!(start(lab_cfg("deadbeef")).is_err());
        // Valid-length but non-hex is also refused.
        assert!(start(lab_cfg(&"z".repeat(64))).is_err());
    }

    /// **Bounded-queue overflow returns the error variant.** Fill the
    /// queue to capacity, then the next enqueue must return
    /// `Err(RpcResponse)` with code `QUEUE_FULL` and the id echoed —
    /// never block. Keeping `_rx` alive prevents a spurious
    /// Disconnected.
    #[test]
    fn queue_overflow_returns_error_variant() {
        let (tx, _rx) = bounded::<PendingRequest>(QUEUE_CAPACITY);
        let mut receivers = Vec::new();
        for i in 0..QUEUE_CAPACITY {
            match enqueue(&tx, a_request(i as i64)) {
                Ok(rx) => receivers.push(rx),
                Err(_) => panic!("request {i} should fit within capacity"),
            }
        }
        let overflow = enqueue(&tx, a_request(999)).expect_err("must overflow");
        assert_eq!(overflow.error.expect("error").code, dispatch::QUEUE_FULL);
        assert_eq!(overflow.id, Value::from(999));
    }

    /// Wrong token, and an absent token (a non-auth first frame), both
    /// cause the server to close the connection.
    #[test]
    fn bad_token_closes_connection() {
        let token = "a".repeat(64);
        let h = start(lab_cfg(&token)).unwrap();
        let addr = h.local_addr();

        // Wrong token.
        let mut s = TcpStream::connect(addr).unwrap();
        transport::write_frame(&mut s, &transport::auth_frame_body(&"b".repeat(64))).unwrap();
        assert!(
            transport::read_frame(&mut s).is_err(),
            "server must close on wrong token"
        );

        // Absent token: first frame is a JSON-RPC request, not auth.
        let mut s2 = TcpStream::connect(addr).unwrap();
        transport::write_frame(
            &mut s2,
            br#"{"jsonrpc":"2.0","id":1,"method":"module_info"}"#,
        )
        .unwrap();
        assert!(
            transport::read_frame(&mut s2).is_err(),
            "server must close when the first frame isn't the token"
        );
    }

    /// A concurrent second client is closed immediately while the
    /// first is authenticated and active.
    #[test]
    fn second_client_rejected() {
        let token = "c".repeat(64);
        let h = start(lab_cfg(&token)).unwrap();
        let addr = h.local_addr();

        // Client A authenticates and stays open.
        let mut a = TcpStream::connect(addr).unwrap();
        transport::write_frame(&mut a, &transport::auth_frame_body(&token)).unwrap();
        let ack = transport::read_frame(&mut a).expect("A gets an ack");
        assert!(!ack.is_empty());

        // Client B connects while A is active — rejected even with a
        // valid token.
        let mut b = TcpStream::connect(addr).unwrap();
        let _ = transport::write_frame(&mut b, &transport::auth_frame_body(&token));
        assert!(
            transport::read_frame(&mut b).is_err(),
            "second concurrent client must be closed"
        );
    }

    /// Full loop off-process: connect, auth, one request, main-thread
    /// drain, framed response with the id echoed. On the host the
    /// native primitive is a stub, so the response is a well-formed
    /// JSON-RPC error — which still proves framing + auth + queue +
    /// drain + dispatch + response write.
    #[test]
    fn end_to_end_request_response() {
        let token = "d".repeat(64);
        let h = start(lab_cfg(&token)).unwrap();
        let addr = h.local_addr();

        let stop = Arc::new(AtomicBool::new(false));
        let drain_stop = stop.clone();
        let drainer = std::thread::spawn(move || {
            while !drain_stop.load(Ordering::SeqCst) {
                h.drain();
                std::thread::sleep(Duration::from_millis(2));
            }
        });

        let mut s = TcpStream::connect(addr).unwrap();
        transport::write_frame(&mut s, &transport::auth_frame_body(&token)).unwrap();
        let _ack = transport::read_frame(&mut s).unwrap();

        let body = br#"{"jsonrpc":"2.0","id":42,"method":"module_info"}"#;
        transport::write_frame(&mut s, body).unwrap();
        let resp = transport::read_frame(&mut s).unwrap();
        let v: Value = serde_json::from_slice(&resp).unwrap();
        assert_eq!(v["id"], Value::from(42));
        assert!(v.get("result").is_some() || v.get("error").is_some());

        stop.store(true, Ordering::SeqCst);
        drainer.join().unwrap();
    }
}
