//! Fetches the cooked-data index and category bodies from the login server.
//!
//! Every request goes to the login server the launcher was given, never a
//! redirect target, and every failure is named by its reason, its HTTP status
//! and, for a transport failure, the OS detail. The timers bound a silent
//! server (the index as a whole, a body per chunk) but never a body that is
//! still moving (D-LP25).

use std::collections::HashSet;
use std::future::Future;
use std::time::Duration;

use cimmeria_cooked_pak::catalog::{decode_body, CategoryBody, Index, Limits};
use cimmeria_cooked_pak::digest::parse_digest_hex;
use reqwest::header::{HeaderMap, RETRY_AFTER};

use super::io_detail::{io_detail_in, IoDetail};

/// The most bytes the index may hold.
pub const INDEX_CAP: usize = 256 * 1024;
/// The most bytes one category body may hold.
pub const BODY_CAP: usize = 32 * 1024 * 1024;
/// The most bytes read from an error response, enough for its `error` code.
const ERROR_BODY_CAP: usize = 4 * 1024;
/// How long the TCP and TLS connect may take.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// How long the whole index request may take.
pub const INDEX_TIMEOUT: Duration = Duration::from_secs(15);
/// How long a body may go without a byte. There is no total limit (D-LP25).
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(15);

/// The time limits one fetcher works under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// The connect limit.
    pub connect: Duration,
    /// The limit on the whole index request.
    pub index: Duration,
    /// The limit on the wait for each body chunk.
    pub idle: Duration,
}

impl Timeouts {
    /// The limits the presync ships with.
    pub const DEFAULT: Timeouts = Timeouts {
        connect: CONNECT_TIMEOUT,
        index: INDEX_TIMEOUT,
        idle: IDLE_TIMEOUT,
    };
}

/// The base of the presync routes for a login URL: the URL without a trailing
/// `/`, then `/api/cooked/v1`.
pub fn base_url(login_url: &str) -> String {
    format!("{}/api/cooked/v1", login_url.trim_end_matches('/'))
}

/// The `host:port` of a login URL, for the UI. The default port is named when
/// the URL has none. `None` when either part is missing.
pub fn server_label(login_url: &str) -> Option<String> {
    let url = reqwest::Url::parse(login_url).ok()?;
    let host = url.host_str()?;
    let port = url.port_or_known_default()?;
    Some(format!("{host}:{port}"))
}

/// Why a fetch did not produce its value.
#[derive(Debug, Clone, PartialEq)]
pub enum FetchError {
    /// The login URL is not one of the login servers the launcher may use.
    EndpointRefused,
    /// The connection failed, with the OS detail when there was one.
    ConnectFailed(Option<IoDetail>),
    /// A limit ran out: the connect or the index, or a body went silent.
    Timeout,
    /// The server has no presync route (404 on the index).
    ServerUnsupported,
    /// The server has the presync switched off (503 `disabled`).
    Disabled,
    /// The server is still building its catalog (503).
    CatalogUnavailable { retry_after_s: Option<u32> },
    /// The server's catalog build failed (503 `catalog_failed`).
    CatalogFailed,
    /// The server is limiting this address (429).
    RateLimited { retry_after_s: Option<u32> },
    /// The category changed on the server while it was fetched (409).
    VersionChanged,
    /// The server has no such category (404 on a body).
    UnknownCategory,
    /// Any other error status.
    HttpError(u16),
    /// The response is over its cap.
    TooLarge,
    /// The index is not a schema 1 index that the launcher can use.
    BadIndex,
    /// The body does not decode, or it is for another category or version.
    BadBody,
}

impl FetchError {
    /// The snake_case reason the report names.
    pub fn reason(&self) -> &'static str {
        match self {
            FetchError::EndpointRefused => "endpoint_refused",
            FetchError::ConnectFailed(_) => "connect_failed",
            FetchError::Timeout => "timeout",
            FetchError::ServerUnsupported => "server_unsupported",
            FetchError::Disabled => "disabled",
            FetchError::CatalogUnavailable { .. } => "catalog_unavailable",
            FetchError::CatalogFailed => "catalog_failed",
            FetchError::RateLimited { .. } => "rate_limited",
            FetchError::VersionChanged => "version_changed",
            FetchError::UnknownCategory => "unknown_category",
            FetchError::HttpError(_) => "http_error",
            FetchError::TooLarge => "too_large",
            FetchError::BadIndex => "bad_index",
            FetchError::BadBody => "bad_body",
        }
    }

    /// The HTTP status of the error, when it came from a response.
    pub fn http_status(&self) -> Option<u16> {
        match self {
            FetchError::ServerUnsupported | FetchError::UnknownCategory => Some(404),
            FetchError::Disabled
            | FetchError::CatalogFailed
            | FetchError::CatalogUnavailable { .. } => Some(503),
            FetchError::RateLimited { .. } => Some(429),
            FetchError::VersionChanged => Some(409),
            FetchError::HttpError(status) => Some(*status),
            _ => None,
        }
    }

    /// The `Retry-After` the server gave, in whole seconds, when it gave one.
    pub fn retry_after_s(&self) -> Option<u32> {
        match self {
            FetchError::CatalogUnavailable { retry_after_s }
            | FetchError::RateLimited { retry_after_s } => *retry_after_s,
            _ => None,
        }
    }

    /// The OS detail of a connect failure, when there was one.
    pub fn io(&self) -> Option<IoDetail> {
        match self {
            FetchError::ConnectFailed(detail) => *detail,
            _ => None,
        }
    }
}

/// Fetches the index and the bodies from one login server.
pub struct Fetcher {
    client: reqwest::Client,
    base: String,
    timeouts: Timeouts,
}

impl Fetcher {
    /// A fetcher under the shipped time limits.
    pub fn new(login_url: &str, login_servers: &[String]) -> Result<Self, FetchError> {
        Self::with_timeouts(login_url, login_servers, Timeouts::DEFAULT)
    }

    /// A fetcher under the given time limits. The login URL must share its
    /// scheme, host and port with one of the login servers.
    pub fn with_timeouts(
        login_url: &str,
        login_servers: &[String],
        timeouts: Timeouts,
    ) -> Result<Self, FetchError> {
        if !origin_allowed(login_url, login_servers) {
            return Err(FetchError::EndpointRefused);
        }
        let client = reqwest::Client::builder()
            .connect_timeout(timeouts.connect)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| FetchError::ConnectFailed(io_detail_in(&error)))?;
        Ok(Self {
            client,
            base: base_url(login_url),
            timeouts,
        })
    }

    /// The category index, checked against schema 1.
    pub async fn index(&self) -> Result<Index, FetchError> {
        let url = format!("{}/index", self.base);
        within(self.timeouts.index, self.fetch_index(&url)).await
    }

    /// The index request and its read, under the index limit.
    async fn fetch_index(&self, url: &str) -> Result<Index, FetchError> {
        let idle = self.timeouts.idle;
        let mut response = self.send(url).await?;
        if !response.status().is_success() {
            return Err(status_error(&mut response, true, idle).await);
        }
        let bytes = read_capped(&mut response, INDEX_CAP, idle).await?;
        let index: Index = serde_json::from_slice(&bytes).map_err(|_| FetchError::BadIndex)?;
        check_index(&index)?;
        Ok(index)
    }

    /// One category body at the given version, and its size in bytes.
    pub async fn body(
        &self,
        category_id: u32,
        version: u32,
    ) -> Result<(CategoryBody, u64), FetchError> {
        let url = format!("{}/category/{category_id}?version={version}", self.base);
        let idle = self.timeouts.idle;
        let mut response = within(idle, self.send(&url)).await?;
        if !response.status().is_success() {
            return Err(status_error(&mut response, false, idle).await);
        }
        let bytes = read_capped(&mut response, BODY_CAP, idle).await?;
        let body = decode_body(&bytes, &Limits::DEFAULT).map_err(|_| FetchError::BadBody)?;
        if body.category_id != category_id || body.version != version {
            return Err(FetchError::BadBody);
        }
        Ok((body, bytes.len() as u64))
    }

    /// Sends one GET; the caller bounds the wait.
    async fn send(&self, url: &str) -> Result<reqwest::Response, FetchError> {
        self.client
            .get(url)
            .send()
            .await
            .map_err(|error| transport_error(&error))
    }
}

/// Whether the login URL's scheme, host and port match a login server's.
fn origin_allowed(login_url: &str, login_servers: &[String]) -> bool {
    let Ok(target) = reqwest::Url::parse(login_url) else {
        return false;
    };
    let target = target.origin();
    login_servers
        .iter()
        .filter_map(|server| reqwest::Url::parse(server).ok())
        .any(|server| server.origin() == target)
}

/// Runs one step under a limit; the limit running out is a `Timeout`.
async fn within<T>(
    limit: Duration,
    work: impl Future<Output = Result<T, FetchError>>,
) -> Result<T, FetchError> {
    tokio::time::timeout(limit, work)
        .await
        .unwrap_or(Err(FetchError::Timeout))
}

/// Reads a response body up to `cap` bytes, waiting at most `idle` for each
/// chunk. Going over the cap is `TooLarge`.
async fn read_capped(
    response: &mut reqwest::Response,
    cap: usize,
    idle: Duration,
) -> Result<Vec<u8>, FetchError> {
    let mut out = Vec::new();
    loop {
        match within(idle, async {
            response.chunk().await.map_err(|e| transport_error(&e))
        })
        .await?
        {
            None => return Ok(out),
            Some(chunk) => {
                if out.len() + chunk.len() > cap {
                    return Err(FetchError::TooLarge);
                }
                out.extend_from_slice(&chunk);
            }
        }
    }
}

/// Maps a non-success response to its error. The 503 and 429 bodies carry an
/// `error` code; an unreadable body counts as no code.
async fn status_error(
    response: &mut reqwest::Response,
    on_index: bool,
    idle: Duration,
) -> FetchError {
    let status = response.status().as_u16();
    let retry_after_s = retry_after(response.headers());
    let code = if matches!(status, 429 | 503) {
        read_capped(response, ERROR_BODY_CAP, idle)
            .await
            .ok()
            .and_then(|bytes| error_code(&bytes))
    } else {
        None
    };
    match (status, code.as_deref()) {
        (404, _) if on_index => FetchError::ServerUnsupported,
        (404, _) => FetchError::UnknownCategory,
        (503, Some("disabled")) => FetchError::Disabled,
        (503, Some("catalog_failed")) => FetchError::CatalogFailed,
        (503, _) => FetchError::CatalogUnavailable { retry_after_s },
        (429, _) => FetchError::RateLimited { retry_after_s },
        (409, _) => FetchError::VersionChanged,
        (other, _) => FetchError::HttpError(other),
    }
}

/// A transport failure: a timeout, or else the connect or read failure with
/// its OS detail.
fn transport_error(error: &reqwest::Error) -> FetchError {
    if error.is_timeout() {
        FetchError::Timeout
    } else {
        FetchError::ConnectFailed(io_detail_in(error))
    }
}

/// The `Retry-After` header as whole seconds. An HTTP date or junk is `None`.
fn retry_after(headers: &HeaderMap) -> Option<u32> {
    headers.get(RETRY_AFTER)?.to_str().ok()?.trim().parse().ok()
}

/// The `error` code of a JSON error body.
fn error_code(bytes: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    value.get("error")?.as_str().map(str::to_owned)
}

/// Checks the index: schema 1, unique category ids, a `.pak` name made of
/// letters and digits (1 to 64 of them) and a digest that parses.
fn check_index(index: &Index) -> Result<(), FetchError> {
    if index.schema != 1 {
        return Err(FetchError::BadIndex);
    }
    let mut seen = HashSet::new();
    for entry in &index.categories {
        let well_formed = seen.insert(entry.category_id)
            && is_pak_name(&entry.pak)
            && parse_digest_hex(&entry.digest).is_some();
        if !well_formed {
            return Err(FetchError::BadIndex);
        }
    }
    Ok(())
}

/// Whether a name is `^[A-Za-z0-9]{1,64}\.pak$`, checked by hand.
fn is_pak_name(name: &str) -> bool {
    match name.strip_suffix(".pak") {
        Some(stem) => {
            (1..=64).contains(&stem.len()) && stem.bytes().all(|b| b.is_ascii_alphanumeric())
        }
        None => false,
    }
}

#[cfg(test)]
#[path = "fetch_tests.rs"]
mod tests;
