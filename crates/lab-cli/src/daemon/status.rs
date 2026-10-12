//! The daemon status, as `lab status` shows it.

use std::fmt;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use super::{paths, token};

/// Where the daemon listens and the bearer token that opens it.
#[derive(Clone)]
pub struct Endpoint {
    /// `http://<bind>`, no trailing slash.
    pub base_url: String,
    /// The bearer token; `None` when unset.
    pub token: Option<String>,
}

impl fmt::Debug for Endpoint {
    // The token is never printed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Endpoint")
            .field("base_url", &self.base_url)
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl Endpoint {
    /// The bind from `labd.pid` (default `127.0.0.1:8779`) and the token
    /// from [`token::daemon_token`].
    pub fn discover() -> Self {
        let info = paths::daemon_info();
        Endpoint {
            base_url: format!("http://{}", info.bind),
            token: token::daemon_token(),
        }
    }
}

/// The body of `GET /status`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct StatusReply {
    /// The daemon process.
    pub daemon: DaemonRow,
    /// One row per hosted lab instance.
    pub instances: Vec<InstanceRow>,
}

/// The `daemon` object.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DaemonRow {
    /// The daemon's process id.
    pub pid: u32,
    /// The daemon's version.
    pub version: String,
    /// When the daemon started (RFC 3339).
    #[serde(default)]
    pub started_at: Option<String>,
    /// Seconds since it started.
    #[serde(default)]
    pub uptime_s: Option<u64>,
}

/// One entry of `instances`. `lease` is the daemon's lease status object
/// (`held`, `lease`, `recent`, `rules`); `uat` is the running `lab_uat_run`
/// progress or `null`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct InstanceRow {
    /// The instance label (`default`, `p2`, ...).
    pub instance: String,
    /// The account the instance logs in with, when it has one.
    #[serde(default)]
    pub account: Option<String>,
    /// The telemetry bridge port.
    #[serde(default)]
    pub bridge_port: Option<u16>,
    /// The game client's process id, when it runs.
    #[serde(default)]
    pub client_pid: Option<u32>,
    /// The lease status object.
    #[serde(default)]
    pub lease: Option<Value>,
    /// The running `lab_uat_run` progress, when there is one.
    #[serde(default)]
    pub uat: Option<Value>,
}

/// Checks the shape the way `Get-LabStatus` does (a `daemon` object and an
/// `instances` key), then deserializes. `None` for anything else.
pub fn parse_status(body: &Value) -> Option<StatusReply> {
    if !has_status_shape(body) {
        return None;
    }
    serde_json::from_value(body.clone()).ok()
}

/// Whether `body` has the shape of `GET /status`: an object with a `daemon`
/// object and an `instances` key (`Get-LabStatus`'s check).
pub fn has_status_shape(body: &Value) -> bool {
    body.as_object().is_some_and(|object| {
        object.get("daemon").is_some_and(Value::is_object) && object.contains_key("instances")
    })
}

/// The time [`fetch`] allows for the whole request, response body included
/// (`-TimeoutSec 5` of `Get-LabStatus`).
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

/// `GET <base_url>/status` with `Authorization: Bearer <token>` and a 5 s
/// timeout ([`FETCH_TIMEOUT`]). `None` when there is no token (no request is
/// made), on any connection, status, timeout or parse failure, and on the
/// wrong shape. Ignores proxy settings (the daemon is on loopback). Never
/// reports the token. Port of `Get-LabStatus`.
pub fn fetch(ep: &Endpoint) -> Option<StatusReply> {
    fetch_with_timeout(ep, FETCH_TIMEOUT)
}

/// [`fetch`] with the request timeout given (so tests need not wait 5 s).
pub fn fetch_with_timeout(ep: &Endpoint, timeout: Duration) -> Option<StatusReply> {
    parse_status(&fetch_value_with_timeout(ep, timeout)?)
}

/// [`fetch`] without the typed parse: the `/status` body as JSON, so a caller
/// can tell an absent key from a `null` one (`lab watch` reads an instance row
/// with no `uat` key as an old daemon). `None` in every case `fetch` is `None`.
pub fn fetch_value(ep: &Endpoint) -> Option<Value> {
    fetch_value_with_timeout(ep, FETCH_TIMEOUT)
}

/// [`fetch_value`] with the request timeout given (see [`fetch_with_timeout`]).
pub fn fetch_value_with_timeout(ep: &Endpoint, timeout: Duration) -> Option<Value> {
    let token = ep.token.as_deref().filter(|t| !t.is_empty())?;
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .no_proxy()
        .build()
        .ok()?;
    let url = format!("{}/status", ep.base_url.trim_end_matches('/'));
    let response = client.get(url).bearer_auth(token).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let text = response.text().ok()?;
    let body: Value = serde_json::from_str(&text).ok()?;
    has_status_shape(&body).then_some(body)
}
