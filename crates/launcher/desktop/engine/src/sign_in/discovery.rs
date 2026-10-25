//! Finds the sign-in server for the login server the launcher is set to (C9).
//!
//! Auth lives on port 443 of the same host as the login server (D-CT4), so the
//! auth base is `https://<host>`. A login server given as an IP literal has no
//! certificate that can name it, so it gets no sign-in. The probe is
//! `GET /auth/v1/providers` (C6); a reply with an `api` other than 1 counts as
//! no sign-in, so an older launcher never reads a newer shape. `client()` is the
//! one HTTP client for sign-in; in debug builds `CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM`
//! (a PEM file path) adds a trusted root, and `CIMMERIA_LAUNCHER_AUTH_BASE`
//! replaces the derived base.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_launcher_core::client_setup::login_session::Links;
use serde::Deserialize;
use url::{Host, Url};

/// How long the probe may take before the server counts as offline.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(4);
/// How long a successful probe is reused for the same login server.
pub const CACHE_TTL: Duration = Duration::from_secs(10 * 60);
/// The only `api` version this launcher reads.
const API_VERSION: u32 = 1;
/// Every sign-in request's limit (C9 session: 10 s). The probe overrides it with [`PROBE_TIMEOUT`].
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// The largest sign-in reply the launcher reads (the relay caps SOAP at 256 KiB; JSON needs far less).
pub const MAX_BODY: usize = 64 * 1024;
/// Debug builds only: path of a PEM file whose root certificate the sign-in client and relay also trust.
const EXTRA_ROOT_ENV: &str = "CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM";
/// Debug builds only: replaces the derived auth base (for a local server).
const AUTH_BASE_ENV: &str = "CIMMERIA_LAUNCHER_AUTH_BASE";

/// One identity provider the server offers, as the sign-in page lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderView {
    pub id: String,
    pub display_name: String,
}

/// What the login server's sign-in service said about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    pub auth_base: Url,
    pub soap_relay: bool,
    pub signup_open: bool,
    pub providers: Vec<ProviderView>,
    pub links: Links,
}

/// The auth base for a login server URL: `https://<host>` with no port, path or query.
///
/// `None` for an IP-literal host (it has no certificate to check) and for a URL
/// without a host. In debug builds `CIMMERIA_LAUNCHER_AUTH_BASE` replaces the
/// result.
pub fn auth_base(login_url: &str) -> Option<Url> {
    if cfg!(debug_assertions) {
        if let Ok(over) = std::env::var(AUTH_BASE_ENV) {
            return Url::parse(&over).ok();
        }
    }
    let url = Url::parse(login_url).ok()?;
    let host = match url.host()? {
        Host::Domain(name) => name.to_owned(),
        Host::Ipv4(_) | Host::Ipv6(_) => return None,
    };
    Url::parse(&format!("https://{host}/")).ok()
}

/// Debug builds only: the bytes of the PEM file `CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM`
/// names, so a local server on a private CA can be reached. `None` in release
/// builds, when the variable is unset, or when the file cannot be read.
pub fn extra_root_pem() -> Option<Vec<u8>> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let path = std::env::var_os(EXTRA_ROOT_ENV)?;
    match std::fs::read(path) {
        Ok(pem) => Some(pem),
        Err(e) => {
            tracing::debug!(kind = ?e.kind(), "sign-in: the extra root file could not be read");
            None
        }
    }
}

/// The HTTP client every sign-in request uses (discovery, the session, the
/// driver): rustls, TLS 1.2 minimum, no redirects, [`REQUEST_TIMEOUT`], plus the
/// debug extra root when it loads.
pub fn client() -> reqwest::Client {
    if let Some(pem) = extra_root_pem() {
        match reqwest::Certificate::from_pem(&pem)
            .and_then(|root| client_builder().add_root_certificate(root).build())
        {
            Ok(client) => return client,
            Err(_) => {
                tracing::warn!(
                    "sign-in: the extra root did not load; using the built-in roots only"
                )
            }
        }
    }
    client_builder()
        .build()
        .expect("the sign-in client builds with the built-in roots")
}

fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .use_rustls_tls()
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(REQUEST_TIMEOUT)
}

/// Reads a reply body up to `cap` bytes. A longer body or a read error gives `None`.
pub(crate) async fn read_capped(mut response: reqwest::Response, cap: usize) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if body.len() + chunk.len() > cap {
            tracing::debug!("sign-in: reply too large");
            return None;
        }
        body.extend_from_slice(&chunk);
    }
    Some(body)
}

/// Asks the login server's sign-in service what it offers. Any failure gives `None`.
pub async fn probe(http: &reqwest::Client, login_url: &str) -> Option<Discovery> {
    let base = match auth_base(login_url) {
        Some(base) => base,
        None => {
            tracing::debug!("sign-in discovery: login server has no auth base");
            return None;
        }
    };
    probe_at(http, base).await
}

/// The probe against a known auth base. `probe` derives the base; tests pass a local one.
pub(crate) async fn probe_at(http: &reqwest::Client, base: Url) -> Option<Discovery> {
    let url = base.join("auth/v1/providers").ok()?;
    let response = match http.get(url).timeout(PROBE_TIMEOUT).send().await {
        Ok(response) => response,
        Err(_) => {
            tracing::debug!("sign-in discovery: server unreachable");
            return None;
        }
    };
    if !response.status().is_success() {
        tracing::debug!("sign-in discovery: status {}", response.status().as_u16());
        return None;
    }
    let body = read_capped(response, MAX_BODY).await?;
    parse(&body, base)
}

/// Reads a `/providers` body. An unknown `api` or a malformed body gives `None`.
pub(crate) fn parse(body: &[u8], auth_base: Url) -> Option<Discovery> {
    let wire: Wire = match serde_json::from_slice(body) {
        Ok(wire) => wire,
        Err(_) => {
            tracing::debug!("sign-in discovery: body is not a providers reply");
            return None;
        }
    };
    if wire.api != API_VERSION {
        tracing::debug!("sign-in discovery: unsupported api {}", wire.api);
        return None;
    }
    let links = wire.links.unwrap_or_default();
    Some(Discovery {
        auth_base,
        soap_relay: wire.soap_relay,
        signup_open: wire.signup_open,
        providers: wire
            .providers
            .into_iter()
            .map(|p| ProviderView {
                id: p.id,
                display_name: p.display_name,
            })
            .collect(),
        links: Links {
            website: links.website,
            account: links.account,
            forums: links.forums,
        },
    })
}

/// The wire shape of `GET /auth/v1/providers` (C6).
#[derive(Deserialize)]
struct Wire {
    api: u32,
    soap_relay: bool,
    signup_open: bool,
    providers: Vec<WireProvider>,
    links: Option<WireLinks>,
}

#[derive(Deserialize)]
struct WireProvider {
    id: String,
    display_name: String,
}

#[derive(Default, Deserialize)]
struct WireLinks {
    website: Option<String>,
    account: Option<String>,
    forums: Option<String>,
}

/// Probe results per login server, reused for [`CACHE_TTL`]. Only successful probes
/// are stored, so a server that was down is asked again on the next Play.
#[derive(Debug, Default)]
pub struct DiscoveryCache {
    entries: HashMap<String, (Instant, Discovery)>,
}

impl DiscoveryCache {
    /// The cached discovery for `login_url`, if it was stored less than [`CACHE_TTL`] ago.
    pub fn get(&self, login_url: &str, now: Instant) -> Option<Discovery> {
        let (stored, discovery) = self.entries.get(login_url)?;
        (now.saturating_duration_since(*stored) < CACHE_TTL).then(|| discovery.clone())
    }

    /// Stores `discovery` for `login_url` as of `now`.
    pub fn insert(&mut self, login_url: &str, now: Instant, discovery: Discovery) {
        self.entries.insert(login_url.to_owned(), (now, discovery));
    }
}

/// [`probe`] through the cache: a fresh entry is returned without a request.
pub async fn probe_cached(
    cache: &mut DiscoveryCache,
    http: &reqwest::Client,
    login_url: &str,
    now: Instant,
) -> Option<Discovery> {
    if let Some(discovery) = cache.get(login_url, now) {
        return Some(discovery);
    }
    let discovery = probe(http, login_url).await?;
    cache.insert(login_url, now, discovery.clone());
    Some(discovery)
}
