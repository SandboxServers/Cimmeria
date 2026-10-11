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
    pub daemon: DaemonRow,
    pub instances: Vec<InstanceRow>,
}

/// The `daemon` object.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DaemonRow {
    pub pid: u32,
    pub version: String,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub uptime_s: Option<u64>,
}

/// One entry of `instances`. `lease` is the daemon's lease status object
/// (`held`, `lease`, `recent`, `rules`); `uat` is the running `lab_uat_run`
/// progress or `null`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct InstanceRow {
    pub instance: String,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub bridge_port: Option<u16>,
    #[serde(default)]
    pub client_pid: Option<u32>,
    #[serde(default)]
    pub lease: Option<Value>,
    #[serde(default)]
    pub uat: Option<Value>,
}

/// Checks the shape the way `Get-LabStatus` does (a `daemon` object and an
/// `instances` key), then deserializes. `None` for anything else.
pub fn parse_status(body: &Value) -> Option<StatusReply> {
    let object = body.as_object()?;
    if !object.get("daemon").is_some_and(Value::is_object) || !object.contains_key("instances") {
        return None;
    }
    serde_json::from_value(body.clone()).ok()
}

/// `GET <base_url>/status` with `Authorization: Bearer <token>` and a 5 s
/// timeout. `None` when there is no token (no request is made), on any
/// connection, status, timeout or parse failure, and on the wrong shape.
/// Ignores proxy settings (the daemon is on loopback). Never reports the
/// token. Port of `Get-LabStatus`.
pub fn fetch(ep: &Endpoint) -> Option<StatusReply> {
    let token = ep.token.as_deref().filter(|t| !t.is_empty())?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
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
    parse_status(&body)
}
