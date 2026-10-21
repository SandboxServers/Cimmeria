//! Where a mint tells the launcher to upload (owner directives 1, 5, 6).
//!
//! The endpoint is https or explicit: the operator's env override, an
//! https-proxied `Host`, a one-host dev box, or the built-in https default.
//! There is no localhost or plain-http fallback: a server that has none of
//! those refuses the mint (503 `no_upload_endpoint`) rather than send a remote
//! launcher to its own machine. Nothing here logs a peer address or a derived
//! URL, because a `Host` can be an IP literal.

use std::sync::atomic::{AtomicU64, Ordering};

use axum::http::{header, HeaderMap};

const ENV_NAME: &str = "CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT";
const UNREACHABLE_WARN_WINDOW_SECS: u64 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndpointSource {
    Env,
    RequestHost,
    LoopbackDev,
    Default,
}

impl EndpointSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            EndpointSource::Env => "env",
            EndpointSource::RequestHost => "request_host",
            EndpointSource::LoopbackDev => "loopback_dev",
            EndpointSource::Default => "default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedEndpoint {
    pub url: String,
    pub source: EndpointSource,
}

impl ResolvedEndpoint {
    pub(crate) fn is_https(&self) -> bool {
        has_scheme(&self.url, "https://")
    }
}

/// True when `url` starts with `scheme` (`"https://"`), ignoring ASCII case,
/// so `HTTPS://` is read as https and `HTTP://` as plain http.
fn has_scheme(url: &str, scheme: &str) -> bool {
    url.get(..scheme.len())
        .is_some_and(|s| s.eq_ignore_ascii_case(scheme))
}

/// Directives 1 and 5: the built-in https Cimmeria URL. `None` until LT-41
/// sets it (D-LT17: the colo has no https endpoint yet). Never plain http.
pub(crate) const DEFAULT_UPLOAD_ENDPOINT: Option<&str> = None;

/// `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT`, trimmed; unset **or blank** is `None`.
/// `docker/compose.yml` passes it as `${...:-}`, which sets `""` when the
/// operator's `.env` has no value, and an empty endpoint would fail every
/// upload with a URL error.
pub(super) fn upload_endpoint_env() -> Option<String> {
    std::env::var(ENV_NAME)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// In order: a non-blank env wins (used as given); else a valid Host with
/// `x-forwarded-proto` exactly `https`; else a loopback Host from a loopback
/// peer (a one-host dev box); else `DEFAULT_UPLOAD_ENDPOINT`; else `None`.
/// No other plain-http URL is ever derived.
pub(crate) fn resolve(
    env: Option<&str>,
    headers: &HeaderMap,
    peer_is_loopback: bool,
) -> Option<ResolvedEndpoint> {
    if let Some(url) = env.map(str::trim).filter(|v| !v.is_empty()) {
        return Some(ResolvedEndpoint {
            url: url.to_string(),
            source: EndpointSource::Env,
        });
    }
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .filter(|h| valid_host(h));
    if let Some(host) = host {
        let https = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            == Some("https");
        if https {
            return Some(ResolvedEndpoint {
                url: format!("https://{host}/api/telemetry"),
                source: EndpointSource::RequestHost,
            });
        }
        let url = format!("http://{host}/api/telemetry");
        if peer_is_loopback && is_loopback_url(&url) {
            return Some(ResolvedEndpoint {
                url,
                source: EndpointSource::LoopbackDev,
            });
        }
    }
    DEFAULT_UPLOAD_ENDPOINT.map(|url| ResolvedEndpoint {
        url: url.to_string(),
        source: EndpointSource::Default,
    })
}

/// A DNS name or IPv4 (`[A-Za-z0-9.-]`, 1..=253 chars) or a bracketed IPv6
/// address, plus an optional `:port` of 1..=5 digits. Nothing else, so a
/// hostile Host cannot smuggle a path or userinfo into the derived URL.
pub(crate) fn valid_host(host: &str) -> bool {
    let (name_ok, port) = if let Some(rest) = host.strip_prefix('[') {
        let Some((inner, tail)) = rest.split_once(']') else {
            return false;
        };
        let inner_ok = !inner.is_empty()
            && inner
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.');
        let port = match tail {
            "" => None,
            t => match t.strip_prefix(':') {
                Some(p) => Some(p),
                None => return false,
            },
        };
        (inner_ok, port)
    } else {
        let (name, port) = match host.split_once(':') {
            Some((n, p)) => (n, Some(p)),
            None => (host, None),
        };
        let ok = !name.is_empty()
            && name.len() <= 253
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        (ok, port)
    };
    let port_ok =
        port.is_none_or(|p| (1..=5).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()));
    name_ok && port_ok
}

/// Host part of an `http(s)://host[:port]/...` URL, brackets kept for IPv6.
fn host_of(url: &str) -> Option<&str> {
    let rest = url.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next()?;
    if authority.starts_with('[') {
        let end = authority.find(']')?;
        return Some(&authority[..=end]);
    }
    authority.split(':').next()
}

pub(crate) fn is_loopback_url(url: &str) -> bool {
    let Some(host) = host_of(url) else {
        return false;
    };
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    bare.eq_ignore_ascii_case("localhost")
        || bare
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.to_canonical().is_loopback())
}

/// The URL may be printed only when its host is a DNS name (directive 6), and
/// then only as `scheme://host[:port]`. Userinfo, path, query and fragment are
/// dropped, so a credential or token in the env value never reaches a log.
fn printable(url: &str) -> Option<String> {
    let host = host_of(url)?;
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if host != bare || bare.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let hostport = authority.rsplit('@').next()?;
    Some(format!("{scheme}://{hostport}"))
}

/// The boot WARN reason for the env value, else `None`. An unset env with a
/// `Some` default is normal and INFO only. Any non-blank env that is not an
/// https URL warns, so a mistyped scheme cannot pass silently.
pub(crate) fn boot_warning(env: Option<&str>) -> Option<&'static str> {
    match env.map(str::trim).filter(|v| !v.is_empty()) {
        None => DEFAULT_UPLOAD_ENDPOINT
            .is_none()
            .then_some("upload_endpoint_unset"),
        Some(url) if is_loopback_url(url) => Some("upload_endpoint_loopback"),
        Some(url) if has_scheme(url, "http://") => Some("upload_endpoint_plain_http"),
        Some(url) if has_scheme(url, "https://") => None,
        Some(_) => Some("upload_endpoint_not_https"),
    }
}

/// The boot line(s) for the upload endpoint. The env URL is printed only
/// when its host is a DNS name, and then without userinfo, path or query;
/// otherwise only the source is.
pub(super) fn log_boot(env: Option<&str>) {
    let source = if env.is_some() { "env" } else { "default" };
    let shown = env.and_then(printable);
    let shown = shown.as_deref();
    match boot_warning(env) {
        Some(reason @ "upload_endpoint_unset") => tracing::warn!(
            reason,
            upload_endpoint_source = source,
            "dev-session telemetry: no https upload endpoint is configured; remote mints are refused (503 no_upload_endpoint) until CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT is set or the built-in https default ships"
        ),
        Some(reason @ "upload_endpoint_loopback") => tracing::warn!(
            reason,
            upload_endpoint_source = source,
            upload_endpoint = shown,
            "dev-session telemetry: CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT is loopback; expected a public https URL; remote launchers will replace it"
        ),
        Some(reason @ "upload_endpoint_not_https") => tracing::warn!(
            reason,
            upload_endpoint_source = source,
            upload_endpoint = shown,
            "dev-session telemetry: CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT is not an https URL; the override is used as given; expected https://"
        ),
        Some(reason) => tracing::warn!(
            reason,
            upload_endpoint_source = source,
            upload_endpoint = shown,
            "dev-session telemetry: CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT is plain http; the explicit override is used as given, but uploads cross the network unencrypted; use https"
        ),
        None => tracing::info!(
            upload_endpoint_source = source,
            upload_endpoint = shown,
            "dev-session telemetry: upload endpoint is the built-in https default (or an https-proxied Host); CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT is an optional override"
        ),
    }
}

/// Throttle state for the unreachable-endpoint WARN.
pub(super) struct WarnThrottle {
    last_unix: AtomicU64,
    suppressed: AtomicU64,
}

impl WarnThrottle {
    pub(super) const fn new() -> Self {
        Self {
            last_unix: AtomicU64::new(0),
            suppressed: AtomicU64::new(0),
        }
    }
}

static UNREACHABLE_THROTTLE: WarnThrottle = WarnThrottle::new();

/// At most one WARN per 600 s, plus a `suppressed` count. No URL, no peer.
pub(crate) fn warn_if_unreachable(endpoint: &ResolvedEndpoint, peer_is_loopback: bool) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    warn_unreachable_at(&UNREACHABLE_THROTTLE, now, endpoint, peer_is_loopback);
}

pub(super) fn warn_unreachable_at(
    throttle: &WarnThrottle,
    now_unix: u64,
    endpoint: &ResolvedEndpoint,
    peer_is_loopback: bool,
) {
    if peer_is_loopback || !is_loopback_url(&endpoint.url) {
        return;
    }
    let last = throttle.last_unix.load(Ordering::Relaxed);
    let due = last == 0 || now_unix.saturating_sub(last) >= UNREACHABLE_WARN_WINDOW_SECS;
    if !due
        || throttle
            .last_unix
            .compare_exchange(last, now_unix, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
    {
        throttle.suppressed.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let suppressed = throttle.suppressed.swap(0, Ordering::Relaxed);
    tracing::warn!(
        reason = "upload_endpoint_unreachable_for_peer",
        upload_endpoint_source = endpoint.source.as_str(),
        suppressed,
        "dev-session telemetry: advertised a loopback upload endpoint to a remote launcher; its uploads will go to its own machine and be lost"
    );
}
