//! Loopback SOAP relay for the launcher's sign-in leg, and the sign-in callback listener.
//!
//! The relay accepts only the two Mercury login SOAP posts on `127.0.0.1`, refuses
//! DNS-rebinding and browser-originated requests, and forwards the rest over TLS to the
//! login server. Nothing it logs carries a body, cookie or header value.

pub mod callback;
#[cfg(test)]
mod tests;

use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::{Bytes, Incoming};
use hyper::header::{HeaderName, CONTENT_TYPE, COOKIE, HOST, ORIGIN, SET_COOKIE};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{HeaderMap, Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Semaphore};
use tokio::task::{JoinHandle, JoinSet};
use url::Url;

/// Request bodies above this size get `413`.
const MAX_REQUEST_BODY: usize = 64 * 1024;
/// Upstream responses above this size get `502`.
const MAX_RESPONSE_BODY: usize = 256 * 1024;
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(20);
/// In-flight requests get this long to finish once the relay is asked to stop.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);
const MAX_CONNECTIONS: usize = 64;
/// A connection that is still open after this long is closed, so idle or slow sockets cannot
/// hold a connection slot for ever. Longer than [`UPSTREAM_TIMEOUT`].
const CONNECTION_LIFETIME: Duration = Duration::from_secs(120);
/// A request whose headers are still arriving after this long is cut off, so a
/// slow-header client cannot hold a connection slot for the whole lifetime.
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(5);
const SOAP_PATHS: [&str; 2] = ["/SGWLogin/UserAuth", "/SGWLogin/ServerSelection"];

/// Configuration for [`start`].
pub struct RelayConfig {
    /// `https://host[:port]` of the login server, with no path, query or userinfo.
    pub upstream: Url,
    /// Extra root certificate (PEM) for the upstream. Used in debug builds only.
    pub extra_root_pem: Option<Vec<u8>>,
    /// Reserved for the launch ticket session (D-MR6). Always `None` in this crate.
    pub session: Option<Infallible>,
}

/// A running relay. Stop it with [`RelayHandle::shutdown`].
pub struct RelayHandle {
    port: u16,
    addr: SocketAddr,
    stop: oneshot::Sender<()>,
    join: JoinHandle<()>,
}

/// Errors from [`start`].
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    /// The loopback listener could not be bound.
    #[error("cannot bind the loopback relay: {0}")]
    Bind(#[from] std::io::Error),
    /// The upstream URL was refused.
    #[error("upstream refused: {0}")]
    Upstream(&'static str),
    /// The upstream HTTP client could not be built.
    #[error("cannot build the upstream client: {0}")]
    Client(#[from] reqwest::Error),
}

impl RelayHandle {
    /// The loopback port the relay listens on.
    pub fn soap_port(&self) -> u16 {
        self.port
    }

    /// The relay's base URL, `http://127.0.0.1:<port>`.
    pub fn soap_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Stops accepting, waits at most two seconds for in-flight requests, then returns.
    pub async fn shutdown(self) {
        let _ = self.stop.send(());
        let _ = self.join.await;
    }
}

/// Per-relay counters, reported in the `launcher.relay.stopped` event.
#[derive(Debug, Default)]
struct Counters {
    requests: AtomicU64,
    rejected: AtomicU64,
    upstream_errors: AtomicU64,
}

/// State shared by every connection of one relay.
struct Shared {
    upstream: String,
    host: String,
    client: reqwest::Client,
    counters: Counters,
}

/// Starts the relay on `127.0.0.1:0`, forwarding to `cfg.upstream`.
pub async fn start(cfg: RelayConfig) -> Result<RelayHandle, RelayError> {
    check_upstream(&cfg.upstream, cfg!(any(test, feature = "test-support")))?;
    let client = build_client(&cfg)?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let addr = listener.local_addr()?;
    let port = addr.port();
    let shared = Arc::new(Shared {
        upstream: cfg.upstream.as_str().trim_end_matches('/').to_owned(),
        host: format!("127.0.0.1:{port}"),
        client,
        counters: Counters::default(),
    });
    let (stop, stop_rx) = oneshot::channel();
    let join = tokio::spawn(run(listener, shared, stop_rx));
    tracing::info!(
        event = "launcher.relay.started",
        outcome = "started",
        "relay started"
    );
    Ok(RelayHandle {
        port,
        addr,
        stop,
        join,
    })
}

/// Refuses any upstream that is not `https` with a bare origin. `allow_test_http` admits
/// `http://127.0.0.1:<port>` for the wiremock tests and nothing else.
fn check_upstream(upstream: &Url, allow_test_http: bool) -> Result<(), RelayError> {
    let loopback = upstream.host_str() == Some("127.0.0.1");
    let scheme_ok = upstream.scheme() == "https"
        || (allow_test_http && upstream.scheme() == "http" && loopback);
    if !scheme_ok {
        return Err(RelayError::Upstream("upstream must be https"));
    }
    if upstream.host_str().is_none() {
        return Err(RelayError::Upstream("upstream needs a host"));
    }
    if upstream.path() != "/" || upstream.query().is_some() || upstream.fragment().is_some() {
        return Err(RelayError::Upstream(
            "upstream must have no path, query or fragment",
        ));
    }
    if !upstream.username().is_empty() || upstream.password().is_some() {
        return Err(RelayError::Upstream("upstream must not carry credentials"));
    }
    Ok(())
}

/// Builds the upstream client: TLS 1.2 minimum, IPv4 source, 20 s timeout, no redirects.
fn build_client(cfg: &RelayConfig) -> Result<reqwest::Client, RelayError> {
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .local_address(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
        .timeout(UPSTREAM_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none());
    if cfg!(debug_assertions) {
        if let Some(pem) = &cfg.extra_root_pem {
            builder = builder.add_root_certificate(reqwest::Certificate::from_pem(pem)?);
        }
    }
    Ok(builder.build()?)
}

/// Accepts connections until stopped, then drains in-flight requests for at most
/// [`SHUTDOWN_GRACE`].
async fn run(listener: TcpListener, shared: Arc<Shared>, mut stop: oneshot::Receiver<()>) {
    let mut conns = JoinSet::new();
    let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    loop {
        let permit = tokio::select! {
            _ = &mut stop => break,
            permit = Arc::clone(&slots).acquire_owned() => match permit {
                Ok(permit) => permit,
                Err(_) => break,
            },
        };
        tokio::select! {
            _ = &mut stop => break,
            Some(_) = conns.join_next(), if !conns.is_empty() => {}
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _)) => {
                        let shared = Arc::clone(&shared);
                        conns.spawn(async move {
                            let _permit = permit;
                            let service = service_fn(move |req| {
                                let shared = Arc::clone(&shared);
                                async move { handle(shared, req).await }
                            });
                            let _ = tokio::time::timeout(
                                CONNECTION_LIFETIME,
                                http1::Builder::new()
                                    .timer(TokioTimer::new())
                                    .header_read_timeout(HEADER_READ_TIMEOUT)
                                    .serve_connection(TokioIo::new(stream), service),
                            )
                            .await;
                        });
                    }
                    // Accept can fail under descriptor exhaustion. Back off instead of
                    // spinning on the error.
                    Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
                }
            }
        }
    }
    drop(listener);
    let drain = async { while conns.join_next().await.is_some() {} };
    if tokio::time::timeout(SHUTDOWN_GRACE, drain).await.is_err() {
        conns.abort_all();
    }
    let counters = &shared.counters;
    tracing::info!(
        event = "launcher.relay.stopped",
        outcome = "stopped",
        requests = counters.requests.load(Ordering::Relaxed),
        rejected = counters.rejected.load(Ordering::Relaxed),
        upstream_errors = counters.upstream_errors.load(Ordering::Relaxed),
        "relay stopped"
    );
}

/// Applies the C9 rules in order (path and method, `Host`, `Origin`, body size) and
/// forwards the accepted requests.
async fn handle(
    shared: Arc<Shared>,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    shared.counters.requests.fetch_add(1, Ordering::Relaxed);
    let (parts, body) = req.into_parts();
    let Some(path) = SOAP_PATHS.iter().copied().find(|p| *p == parts.uri.path()) else {
        return Ok(refuse(&shared, StatusCode::NOT_FOUND));
    };
    if parts.method != Method::POST {
        return Ok(refuse(&shared, StatusCode::NOT_FOUND));
    }
    if header_text(&parts.headers, &HOST) != Some(shared.host.as_str()) {
        return Ok(refuse(&shared, StatusCode::MISDIRECTED_REQUEST));
    }
    if parts.headers.contains_key(ORIGIN) {
        return Ok(refuse(&shared, StatusCode::FORBIDDEN));
    }
    let payload = match Limited::new(body, MAX_REQUEST_BODY).collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) if err.downcast_ref::<LengthLimitError>().is_some() => {
            return Ok(refuse(&shared, StatusCode::PAYLOAD_TOO_LARGE));
        }
        Err(_) => return Ok(refuse(&shared, StatusCode::BAD_REQUEST)),
    };
    Ok(forward(&shared, path, &parts.headers, payload).await)
}

/// Sends the accepted request upstream and copies back status, body, `Content-Type` and
/// `Set-Cookie`. Any upstream failure or oversized reply is `502` with an empty body.
async fn forward(
    shared: &Shared,
    path: &str,
    headers: &HeaderMap,
    payload: Bytes,
) -> Response<Full<Bytes>> {
    let mut request = shared
        .client
        .post(format!("{}{}", shared.upstream, path))
        .body(payload);
    for name in [CONTENT_TYPE, HeaderName::from_static("soapaction"), COOKIE] {
        if let Some(value) = headers.get(&name) {
            request = request.header(name, value.clone());
        }
    }
    let Ok(mut upstream) = request.send().await else {
        return upstream_failed(shared);
    };
    let status = upstream.status();
    let content_type = upstream.headers().get(CONTENT_TYPE).cloned();
    let cookies: Vec<_> = upstream
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .cloned()
        .collect();
    let mut reply = Vec::new();
    loop {
        match upstream.chunk().await {
            Ok(Some(chunk)) if reply.len() + chunk.len() <= MAX_RESPONSE_BODY => {
                reply.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            _ => return upstream_failed(shared),
        }
    }
    let mut builder = Response::builder().status(status);
    if let Some(content_type) = content_type {
        builder = builder.header(CONTENT_TYPE, content_type);
    }
    for cookie in cookies {
        builder = builder.header(SET_COOKIE, cookie);
    }
    match builder.body(Full::new(Bytes::from(reply))) {
        Ok(response) => response,
        Err(_) => upstream_failed(shared),
    }
}

fn upstream_failed(shared: &Shared) -> Response<Full<Bytes>> {
    shared
        .counters
        .upstream_errors
        .fetch_add(1, Ordering::Relaxed);
    empty(StatusCode::BAD_GATEWAY)
}

fn refuse(shared: &Shared, status: StatusCode) -> Response<Full<Bytes>> {
    shared.counters.rejected.fetch_add(1, Ordering::Relaxed);
    empty(status)
}

fn empty(status: StatusCode) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::new()));
    *response.status_mut() = status;
    response
}

fn header_text<'a>(headers: &'a HeaderMap, name: &HeaderName) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}
