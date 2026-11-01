//! A fake client bridge for flow tests: a loopback TCP server that speaks
//! the bridge's framed JSON-RPC (4-byte LE length, token handshake, one
//! request → one response) and answers each call from a test closure.
//!
//! Only flows that never post window messages can run against it (input
//! needs the game's HWND), which is why the combat flows focus the window
//! lazily, right before the first key or click.

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::client::BridgeClient;
use crate::supervisor::{Supervisor, SupervisorConfig};

/// Answers one call: `Ok(result)` or `Err(message)` (a bridge error).
pub type Responder = Arc<dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync>;

/// Answers one call with an optional C4 `meta` stamp for the response.
pub type MetaResponder =
    Arc<dyn Fn(&str, &Value) -> Result<(Value, Option<Value>), String> + Send + Sync>;

pub const TOKEN: &str = "fake-bridge-token";

async fn read_frame(s: &mut TcpStream) -> Option<Vec<u8>> {
    let mut len = [0u8; 4];
    s.read_exact(&mut len).await.ok()?;
    let mut body = vec![0u8; u32::from_le_bytes(len) as usize];
    s.read_exact(&mut body).await.ok()?;
    Some(body)
}

async fn write_frame(s: &mut TcpStream, v: &Value) {
    let body = serde_json::to_vec(v).expect("json");
    let _ = s.write_all(&(body.len() as u32).to_le_bytes()).await;
    let _ = s.write_all(&body).await;
}

/// How a connection answers: plain results, or results with a `meta`.
#[derive(Clone)]
enum Answer {
    Plain(Responder),
    Meta(MetaResponder),
}

impl Answer {
    fn respond(&self, method: &str, params: &Value) -> Result<(Value, Option<Value>), String> {
        match self {
            Answer::Plain(r) => r(method, params).map(|v| (v, None)),
            Answer::Meta(r) => r(method, params),
        }
    }
}

async fn serve(mut s: TcpStream, responder: Answer) {
    let Some(auth) = read_frame(&mut s).await else {
        return;
    };
    let auth: Value = serde_json::from_slice(&auth).unwrap_or(Value::Null);
    let ok = auth["token"] == TOKEN;
    write_frame(&mut s, &json!({ "ok": ok })).await;
    if !ok {
        return;
    }
    while let Some(req) = read_frame(&mut s).await {
        let req: Value = serde_json::from_slice(&req).unwrap_or(Value::Null);
        let method = req["method"].as_str().unwrap_or_default();
        let resp = match responder.respond(method, &req["params"]) {
            Ok((result, meta)) => {
                let mut r = json!({ "jsonrpc": "2.0", "id": req["id"], "result": result });
                if let Some(m) = meta {
                    r["meta"] = m;
                }
                r
            }
            Err(e) => json!({ "jsonrpc": "2.0", "id": req["id"],
                              "error": { "code": -32000, "message": e } }),
        };
        write_frame(&mut s, &resp).await;
    }
}

/// Start the fake and return a supervisor wired to it.
pub async fn supervisor(responder: Responder) -> Supervisor {
    supervisor_at(listen(Answer::Plain(responder)).await)
}

/// [`supervisor`] whose responses may carry a C4 `meta` stamp.
#[allow(dead_code)] // The packet's named entry point; tests use `client_with_meta`.
pub async fn supervisor_with_meta(responder: MetaResponder) -> Supervisor {
    supervisor_at(listen(Answer::Meta(responder)).await)
}

/// A bare client wired to a stamping fake, for `client.rs` tests.
pub async fn client_with_meta(responder: MetaResponder) -> BridgeClient {
    BridgeClient::new(listen(Answer::Meta(responder)).await, TOKEN)
}

async fn listen(responder: Answer) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    tokio::spawn(async move {
        while let Ok((s, _)) = listener.accept().await {
            tokio::spawn(serve(s, responder.clone()));
        }
    });
    addr
}

fn supervisor_at(addr: String) -> Supervisor {
    let config = SupervisorConfig {
        install_dir: None,
        dll_path: None,
        patches_dll: None,
        helper_path: None,
        bind: "127.0.0.1".into(),
        port: 8770,
        instance: None,
        telemetry: Default::default(),
    };
    Supervisor::new(Arc::new(BridgeClient::new(addr, TOKEN)), config)
}

/// A `lua_eval` success with these results.
pub fn lua_ok<S: AsRef<str>>(results: &[S]) -> Value {
    json!({ "ok": true, "status": 0, "error": "",
            "results": results.iter().map(|r| r.as_ref()).collect::<Vec<_>>() })
}

/// An `events_read` batch.
pub fn events(batch: Vec<(&str, i64, Value)>) -> Value {
    json!({
        "returned": batch.len(),
        "dropped": 0,
        "events": batch.into_iter()
            .map(|(k, t, f)| json!({ "kind": k, "ts_ms": t, "fields": f }))
            .collect::<Vec<_>>(),
    })
}

/// The Lua ring read of an empty, installed ring.
pub fn empty_rings() -> Value {
    lua_ok(&[
        "epoch\tE1",
        "install\tcombat\tok",
        "install\tchat\tok",
        "head\tcombat\t0",
        "head\tchat\t0",
    ])
}

/// The chunk is the lab's ring pump.
pub fn is_ring_pump(params: &Value) -> bool {
    params["chunk"]
        .as_str()
        .is_some_and(|c| c.contains("CimmeriaLab"))
}
