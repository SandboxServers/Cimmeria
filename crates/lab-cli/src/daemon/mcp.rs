//! The client for the daemon's MCP endpoint.
//!
//! A port of `Read-McpBody`, `New-McpSession`, `Send-McpRequest`,
//! `Invoke-McpTool`, `Close-McpSession`, `Wait-UatLeaseFree` and
//! `Hide-LeaseIds` from the removed PowerShell uat library. The protocol is the
//! streamable-HTTP one: `initialize`, the `Mcp-Session-Id` it returns,
//! `notifications/initialized`, then one POST per `tools/call`, whose reply
//! is plain JSON or an SSE stream (the last `data:` line is the answer).
//!
//! The core takes a base URL and an optional token; `McpClient::connect` and
//! `wait_lease_free` are the thin wrappers over a discovered [`Endpoint`].

use std::fmt;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use regex::Regex;
use reqwest::blocking::Client;
use serde_json::{json, Value};

use super::status::{self, Endpoint};

const PROTOCOL_VERSION: &str = "2025-06-18";
const ACCEPT: &str = "application/json, text/event-stream";
const SESSION_HEADER: &str = "Mcp-Session-Id";
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(10);

/// How long `wait_lease_free_at` sleeps between two looks at `/status`
/// (`Start-Sleep -Seconds 10` in `Wait-UatLeaseFree`).
pub const LEASE_POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Why a call to the daemon's MCP endpoint did not give a result.
#[derive(Debug)]
pub enum McpError {
    /// Nothing answered: connection refused, a timeout, any transport error.
    Unreachable(String),
    /// The daemon answered with a non-2xx status (401 and 403 included): status, start of the body.
    Http(u16, String),
    /// The tool ran and said no (`isError`), or the reply was a JSON-RPC error.
    Tool(String),
    /// The reply was not the shape the protocol promises.
    Protocol(String),
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            McpError::Unreachable(m) => write!(f, "cannot reach the lab daemon: {m}"),
            McpError::Http(code, body) => write!(f, "the lab daemon answered HTTP {code}: {body}"),
            McpError::Tool(m) => write!(f, "{m}"),
            McpError::Protocol(m) => write!(f, "unexpected reply from the lab daemon: {m}"),
        }
    }
}

impl std::error::Error for McpError {}

/// One open MCP session on the daemon.
pub struct McpClient {
    http: Client,
    url: String,
    token: Option<String>,
    session_id: Option<String>,
    next_id: u64,
}

impl McpClient {
    /// Opens a session on `base_url` (for example `http://127.0.0.1:8779`):
    /// POST `initialize` (protocol 2025-06-18, client `lab-cli`), keep the
    /// `Mcp-Session-Id` header, POST `notifications/initialized`.
    pub fn open(base_url: &str, token: Option<&str>) -> Result<Self, McpError> {
        // no_proxy: this is loopback, and a system proxy must never see the token.
        let http = Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| McpError::Protocol(format!("http client: {e}")))?;
        let mut client = McpClient {
            http,
            url: format!("{}/mcp", base_url.trim_end_matches('/')),
            token: token.map(str::to_owned),
            session_id: None,
            next_id: 2,
        };
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "lab-cli", "version": env!("CARGO_PKG_VERSION") },
            },
        });
        let text = client.post(&init, HANDSHAKE_TIMEOUT)?;
        if let Some(reply) = read_body(&text)? {
            if let Some(message) = rpc_error_message(&reply) {
                return Err(McpError::Tool(message));
            }
        }
        let notice = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        client.post(&notice, HANDSHAKE_TIMEOUT)?;
        Ok(client)
    }

    /// Calls one tool and returns its first text content parsed as JSON (a
    /// text that is not JSON comes back as a JSON string). `timeout` bounds
    /// this one request.
    pub fn call_tool(
        &mut self,
        name: &str,
        args: Value,
        timeout: Duration,
    ) -> Result<Value, McpError> {
        let id = self.next_id;
        self.next_id += 1;
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": args },
        });
        let text = self.post(&body, timeout)?;
        let reply = read_body(&text)?
            .ok_or_else(|| McpError::Protocol(format!("{name} returned nothing")))?;
        if let Some(message) = rpc_error_message(&reply) {
            return Err(McpError::Tool(message));
        }
        let result = reply
            .get("result")
            .ok_or_else(|| McpError::Protocol(format!("{name} returned no result")))?;
        let content_text = result
            .get("content")
            .and_then(Value::as_array)
            .and_then(|c| c.first())
            .and_then(|c| c.get("text"))
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::Protocol(format!("{name} returned no content")))?;
        if result.get("isError").and_then(Value::as_bool) == Some(true) {
            return Err(McpError::Tool(content_text.to_owned()));
        }
        Ok(serde_json::from_str(content_text)
            .unwrap_or_else(|_| Value::String(content_text.to_owned())))
    }

    /// Ends the session with a DELETE. Best effort: every error is ignored
    /// (the daemon also drops idle sessions). A server that issued no session
    /// id gets no DELETE.
    pub fn close(self) {
        let Some(sid) = &self.session_id else { return };
        let mut req = self
            .http
            .delete(&self.url)
            .header("Accept", ACCEPT)
            .header(SESSION_HEADER, sid)
            .timeout(CLOSE_TIMEOUT);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let _ = req.send();
    }

    /// POSTs one JSON-RPC body; returns the reply text. Records a session id
    /// the reply carries. Maps transport and HTTP failures per the table.
    fn post(&mut self, body: &Value, timeout: Duration) -> Result<String, McpError> {
        let mut req = self
            .http
            .post(&self.url)
            .header("Accept", ACCEPT)
            .header("Content-Type", "application/json")
            .timeout(timeout)
            .body(body.to_string());
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        if let Some(sid) = &self.session_id {
            req = req.header(SESSION_HEADER, sid);
        }
        let resp = req
            .send()
            .map_err(|e| McpError::Unreachable(e.to_string()))?;
        let status = resp.status();
        if let Some(sid) = resp
            .headers()
            .get(SESSION_HEADER)
            .and_then(|v| v.to_str().ok())
        {
            self.session_id = Some(sid.to_owned());
        }
        let text = resp.text().map_err(|e| {
            if e.is_timeout() {
                McpError::Unreachable(e.to_string())
            } else {
                McpError::Protocol(format!("reading the reply: {e}"))
            }
        })?;
        if !status.is_success() {
            return Err(McpError::Http(
                status.as_u16(),
                text.chars().take(200).collect(),
            ));
        }
        Ok(text)
    }
}

/// The message of a JSON-RPC `error` object, `no message` when it has none.
fn rpc_error_message(reply: &Value) -> Option<String> {
    let err = reply.get("error").filter(|e| !e.is_null())?;
    Some(
        err.get("message")
            .and_then(Value::as_str)
            .unwrap_or("no message")
            .to_owned(),
    )
}

/// The JSON-RPC body of a reply: plain JSON, or the last SSE `data:` line.
/// `None` for an empty body or an SSE body with no `data:` line
/// (`Read-McpBody`; a notification's 202 has no body).
fn read_body(text: &str) -> Result<Option<Value>, McpError> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let json_text = if text.trim_start().starts_with('{') {
        text.trim().to_owned()
    } else {
        let Some(line) = text.split('\n').rfind(|l| l.starts_with("data:")) else {
            return Ok(None);
        };
        line[5..].trim().to_owned()
    };
    serde_json::from_str(&json_text)
        .map(Some)
        .map_err(|e| McpError::Protocol(format!("reply is not JSON: {e}")))
}

/// `GET <base_url>/status` as raw JSON through [`status::fetch_value`] (one
/// reader of `/status`, as `Get-LabStatus`): `None` with no token (no request
/// is made), on any failure, and on the wrong shape.
fn fetch_status_value(base_url: &str, token: Option<&str>) -> Option<Value> {
    status::fetch_value(&Endpoint {
        base_url: base_url.to_owned(),
        token: token.map(str::to_owned),
    })
}

/// Waits until `instance`'s lease is free, polling `poll` (a `/status`
/// reader) every `interval` until `max` has passed (`Wait-UatLeaseFree`).
/// True when it cleared in time. A poll that returns `None` (daemon down) is
/// "unknown", not "free": keep waiting. An instance missing from the reply
/// counts as free, as the PowerShell does.
pub fn wait_lease_free_with(
    poll: &mut dyn FnMut() -> Option<Value>,
    instance: &str,
    max: Duration,
    interval: Duration,
) -> bool {
    wait_lease_free_until(
        poll,
        instance,
        max,
        interval,
        &|| false,
        &mut RealClock::default(),
    )
}

/// The longest single sleep inside a lease wait: a stop request is seen within it.
pub const STOP_SLICE: Duration = Duration::from_millis(200);

/// The clock and the sleeper of a lease wait. The real one is [`RealClock`];
/// tests pass a fake one that moves time without sleeping.
pub trait LeaseClock {
    /// Time since the wait began.
    fn elapsed(&mut self) -> Duration;
    /// Sleeps for `d`.
    fn sleep(&mut self, d: Duration);
}

/// [`Instant`] and `thread::sleep`; the wait begins when it is made.
pub struct RealClock(Instant);

impl Default for RealClock {
    fn default() -> Self {
        RealClock(Instant::now())
    }
}

impl LeaseClock for RealClock {
    fn elapsed(&mut self) -> Duration {
        self.0.elapsed()
    }

    fn sleep(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// Whether `instance`'s row of a `/status` body says its lease is held. An
/// instance missing from the body counts as free, as the PowerShell does.
fn lease_held(status: &Value, instance: &str) -> bool {
    status
        .get("instances")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|r| r.get("instance").and_then(Value::as_str) == Some(instance))
        })
        .and_then(|row| row.get("lease"))
        .and_then(|lease| lease.get("held"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// [`wait_lease_free_with`] with a stop check and a clock. `stop` is asked at
/// the top of every round and before every sleep slice (each at most
/// [`STOP_SLICE`] long), so a `true` ends the wait within one slice and the
/// answer is `false` (the lease did not clear). A stop that is already `true`
/// polls nothing. The wait never sleeps past `max`; the poll after the last
/// sleep is the last look.
pub fn wait_lease_free_until(
    poll: &mut dyn FnMut() -> Option<Value>,
    instance: &str,
    max: Duration,
    interval: Duration,
    stop: &dyn Fn() -> bool,
    clock: &mut dyn LeaseClock,
) -> bool {
    loop {
        if stop() {
            return false;
        }
        if let Some(status) = poll() {
            if !lease_held(&status, instance) {
                return true;
            }
        }
        let left = max.saturating_sub(clock.elapsed());
        if left.is_zero() {
            return false;
        }
        // Slices are cut from the clock, not from a running sum of the nominal
        // slice lengths: a sleep that overshoots (about 15 ms on Windows) is
        // not added up over fifty slices, so the poll cadence and `max` hold.
        let until = clock.elapsed().saturating_add(interval.min(left));
        loop {
            if stop() {
                return false;
            }
            let now = clock.elapsed();
            if now >= until {
                break;
            }
            clock.sleep((until - now).min(STOP_SLICE));
        }
    }
}

/// `wait_lease_free_with` reading `GET /status` on `base_url` every `interval`.
pub fn wait_lease_free_at_every(
    base_url: &str,
    token: Option<&str>,
    instance: &str,
    max: Duration,
    interval: Duration,
) -> bool {
    wait_lease_free_with(
        &mut || fetch_status_value(base_url, token),
        instance,
        max,
        interval,
    )
}

/// `wait_lease_free_at_every` at the PowerShell's 10 second interval.
pub fn wait_lease_free_at(
    base_url: &str,
    token: Option<&str>,
    instance: &str,
    max: Duration,
) -> bool {
    wait_lease_free_at_every(base_url, token, instance, max, LEASE_POLL_INTERVAL)
}

/// [`wait_lease_free_at_every`] that also stops: `stop` is asked as in
/// [`wait_lease_free_until`] and a `true` ends the wait with `false`.
pub fn wait_lease_free_stoppable_every(
    base_url: &str,
    token: Option<&str>,
    instance: &str,
    max: Duration,
    interval: Duration,
    stop: &dyn Fn() -> bool,
) -> bool {
    wait_lease_free_until(
        &mut || fetch_status_value(base_url, token),
        instance,
        max,
        interval,
        stop,
        &mut RealClock::default(),
    )
}

impl McpClient {
    /// [`McpClient::open`] over a discovered endpoint.
    pub fn connect(ep: &Endpoint) -> Result<Self, McpError> {
        Self::open(&ep.base_url, ep.token.as_deref())
    }
}

/// Port of `Wait-UatLeaseFree`: true when `instance`'s lease is free within `max`,
/// polling `GET /status` on the endpoint at the PowerShell's 10 second interval.
pub fn wait_lease_free(ep: &Endpoint, instance: &str, max: Duration) -> bool {
    wait_lease_free_at(&ep.base_url, ep.token.as_deref(), instance, max)
}

/// [`wait_lease_free`] that ends at once when `stop` turns `true` (the lab
/// uat batch passes `ctrlc::stop_requested`): the answer is then `false`.
pub fn wait_lease_free_stoppable(
    ep: &Endpoint,
    instance: &str,
    max: Duration,
    stop: &dyn Fn() -> bool,
) -> bool {
    wait_lease_free_stoppable_every(
        &ep.base_url,
        ep.token.as_deref(),
        instance,
        max,
        LEASE_POLL_INTERVAL,
        stop,
    )
}

/// Lease ids (`lease-<hex>`) replaced in text that leaves the command
/// (`Hide-LeaseIds`). Matches ignoring case, as PowerShell's `-replace` does.
pub fn hide_lease_ids(text: &str) -> String {
    static LEASE_ID: OnceLock<Regex> = OnceLock::new();
    let re =
        LEASE_ID.get_or_init(|| Regex::new(r"(?i)lease-[0-9a-f]{8,}").expect("lease id regex"));
    re.replace_all(text, "lease-<redacted>").into_owned()
}
