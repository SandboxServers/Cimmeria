//! Auth-server TLS termination for the SOAP login service.
//!
//! The auth login is a SOAP POST that historically crosses the network over
//! **plain HTTP** (see `docs/architecture/encryption-modernization.md`). Phase 1
//! adds a TLS listener that runs **in parallel** with the plain-HTTP listener
//! during the transition window, serving the **same** axum `Router`.
//!
//! Design notes:
//!
//! - **Why a `Listener` impl, not `axum::serve` + hyper-util.** `axum::serve`
//!   already drives the full hyper stack for any type implementing
//!   [`axum::serve::Listener`]. We wrap the TCP accept + rustls handshake behind
//!   that trait so the serve path is byte-identical to the HTTP listener — same
//!   router, same connect-info, same graceful behaviour. No direct hyper/
//!   hyper-util dependency is needed.
//!
//! - **Why `ArcSwap`.** The active `rustls::ServerConfig` lives behind an
//!   [`arc_swap::ArcSwap`] and is read **per connection** in `accept()`. A
//!   [`TlsCertStore::reload`] re-reads the cert/key files and atomically swaps
//!   the config in. In-flight connections keep the config they handshook with;
//!   new connections pick up the rotated cert — no listener restart, no dropped
//!   socket. The background mtime watcher in [`super::cert_watcher`] calls
//!   `reload` on a poll interval so an operator's cert rotation is picked up
//!   without restarting the server.
//!
//! - **Handshake failures never kill the listener.** Per the `Listener` trait
//!   contract, `accept()` must log-and-retry on error. A client speaking plain
//!   HTTP to the TLS port (or a bad SNI) fails the handshake; we log at `debug`
//!   and loop, exactly as `axum`'s own `TcpListener` impl loops on accept errors.

use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use tokio::net::TcpListener;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::{server::TlsStream, TlsAcceptor};

/// Errors raised while loading or reloading the TLS material.
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("failed to read TLS file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("no certificates found in PEM chain {0}")]
    NoCerts(PathBuf),

    #[error("no private key found in PEM file {0}")]
    NoKey(PathBuf),

    #[error("rustls rejected the cert/key pair: {0}")]
    Rustls(#[from] tokio_rustls::rustls::Error),
}

/// The live, served TLS material: the assembled `rustls::ServerConfig` plus the
/// DER of the leaf certificate it serves.
///
/// Bundling the leaf DER with the config (rather than digging it back out of
/// rustls, which hides its cert chain behind a private `ClientHello`) lets the
/// reload path and tests observe *which* cert is currently live with a cheap
/// byte comparison — proving a hot-swap installed *different* material, not just
/// a fresh-but-identical config.
struct LiveTls {
    config: Arc<ServerConfig>,
    // Only read by the test-only `current_leaf_cert_der` observability hook; in
    // a release build nothing reads it back out, but it is still captured on
    // every (re)build so the hook is always accurate when tests run.
    #[cfg_attr(not(test), allow(dead_code))]
    leaf_cert_der: Vec<u8>,
}

/// Holds the live `rustls::ServerConfig` behind an `ArcSwap` so the cert can be
/// hot-reloaded without restarting the listener.
///
/// Construct with [`TlsCertStore::load`]; rotate with [`TlsCertStore::reload`].
#[derive(Clone)]
pub struct TlsCertStore {
    cert_path: PathBuf,
    key_path: PathBuf,
    live: Arc<ArcSwap<LiveTls>>,
}

impl std::fmt::Debug for TlsCertStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `rustls::ServerConfig` isn't Debug and would leak nothing useful
        // anyway; show the paths so logs/test failures identify the cert.
        f.debug_struct("TlsCertStore")
            .field("cert_path", &self.cert_path)
            .field("key_path", &self.key_path)
            .finish_non_exhaustive()
    }
}

impl TlsCertStore {
    /// Load the PEM cert chain + private key from disk and build the initial
    /// `rustls::ServerConfig`.
    pub fn load(cert_path: impl AsRef<Path>, key_path: impl AsRef<Path>) -> Result<Self, TlsError> {
        let cert_path = cert_path.as_ref().to_path_buf();
        let key_path = key_path.as_ref().to_path_buf();
        let live = build_live_tls(&cert_path, &key_path)?;
        Ok(Self {
            cert_path,
            key_path,
            live: Arc::new(ArcSwap::from_pointee(live)),
        })
    }

    /// Re-read the cert/key files from their original paths and atomically swap
    /// the active `ServerConfig`.
    ///
    /// This is the reload **seam** — the background mtime watcher in
    /// [`super::cert_watcher`] calls this on a poll interval. On error the
    /// previous config is left in place (the swap only happens after a
    /// successful rebuild), so a botched cert rotation can't take the listener
    /// down.
    pub fn reload(&self) -> Result<(), TlsError> {
        let new_live = build_live_tls(&self.cert_path, &self.key_path)?;
        self.live.store(Arc::new(new_live));
        tracing::info!(
            cert = %self.cert_path.display(),
            "Reloaded auth TLS certificate"
        );
        Ok(())
    }

    /// Snapshot the current config (read per-connection in the accept loop).
    fn current(&self) -> Arc<ServerConfig> {
        Arc::clone(&self.live.load().config)
    }

    /// The configured cert chain path. Used by the mtime watcher to stat the
    /// file it must reload from.
    pub(super) fn cert_path(&self) -> &Path {
        &self.cert_path
    }

    /// The configured private key path. Used by the mtime watcher to stat the
    /// file it must reload from.
    pub(super) fn key_path(&self) -> &Path {
        &self.key_path
    }

    /// Return the DER bytes of the leaf certificate the store is *currently*
    /// serving.
    ///
    /// The leaf DER carries the SPKI + serial, so a reload test can prove a
    /// hot-swap installed *different* cert material — not merely that a swap
    /// occurred — by comparing this value across a reload. Captured at build
    /// time and carried alongside the live config (see [`LiveTls`]) so it always
    /// reflects exactly what the listener serves.
    #[cfg(test)]
    pub(super) fn current_leaf_cert_der(&self) -> Vec<u8> {
        self.live.load().leaf_cert_der.clone()
    }
}

/// Read a PEM cert chain + private key, assemble a `rustls::ServerConfig` with
/// no client-cert verification (standard server-auth-only TLS), and capture the
/// leaf certificate's DER for the live-cert observability hook.
///
/// Both the initial [`TlsCertStore::load`] and every [`TlsCertStore::reload`]
/// go through here, so the leaf DER carried in [`LiveTls`] always matches the
/// config actually built — there is no second read that could drift.
///
/// The provider is pinned to **aws-lc-rs** explicitly rather than relying on
/// `ServerConfig::builder()`'s process-default: this crate's dependency graph
/// compiles in *both* the `aws-lc-rs` and `ring` rustls backends (reqwest and
/// other deps each pull one), and with two providers present rustls 0.23
/// refuses to auto-select and panics. Pinning the provider here makes the auth
/// listener deterministic regardless of what the rest of the binary links.
fn build_live_tls(cert_path: &Path, key_path: &Path) -> Result<LiveTls, TlsError> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    // `load_certs` guarantees a non-empty, leaf-first chain.
    let leaf_cert_der = certs[0].as_ref().to_vec();

    let provider = Arc::new(tokio_rustls::rustls::crypto::aws_lc_rs::default_provider());

    // Verify the private key actually matches the leaf certificate BEFORE
    // building the live config. `with_single_cert` does NOT cross-check this,
    // so without the guard a non-atomic rotation (new cert written, old key
    // still on disk for a tick) could install a mismatched pair that fails
    // every TLS handshake until the next reload. On mismatch we error here;
    // `reload` then retains the previous good config and the watcher retries on
    // the next tick once both files are consistent.
    let signing_key = provider.key_provider.load_private_key(key.clone_key())?;
    let certified = tokio_rustls::rustls::sign::CertifiedKey::new(certs.clone(), signing_key);
    certified.keys_match()?;

    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    Ok(LiveTls {
        config: Arc::new(config),
        leaf_cert_der,
    })
}

/// Parse the leaf-first PEM certificate chain.
fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let pem = std::fs::read(path).map_err(|source| TlsError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = io::BufReader::new(&pem[..]);
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| TlsError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if certs.is_empty() {
        return Err(TlsError::NoCerts(path.to_path_buf()));
    }
    Ok(certs)
}

/// Parse the first private key in the PEM file. Accepts PKCS#8, PKCS#1/RSA, and
/// SEC1/EC keys — whichever `rustls-pemfile` finds first.
fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, TlsError> {
    let pem = std::fs::read(path).map_err(|source| TlsError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = io::BufReader::new(&pem[..]);
    // `private_key` returns the first key of any supported kind, or None.
    let key = rustls_pemfile::private_key(&mut reader).map_err(|source| TlsError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    key.ok_or_else(|| TlsError::NoKey(path.to_path_buf()))
}

/// The HSTS header value applied to every auth response.
///
/// `max-age=63072000` is two years (the RFC 6797 / preload-list recommended
/// floor); `includeSubDomains` extends the policy to every subdomain. Stamped
/// on *both* the HTTP and HTTPS paths for wiring simplicity, but per RFC 6797
/// a user agent only **honours** the header when it is received over HTTPS —
/// the plain-HTTP copy has no effect and is not relied upon for protection.
pub const HSTS_VALUE: &str = "max-age=63072000; includeSubDomains";

/// axum middleware that stamps `Strict-Transport-Security` onto every response.
///
/// Applied as a `from_fn` layer on the shared `Router` so both the HTTP and
/// HTTPS listeners emit it. Kept here (next to the TLS code it complements)
/// rather than in `service.rs` so the header value and the listener live
/// together.
pub async fn hsts_layer(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    // HeaderValue::from_static can't fail for a const ASCII string.
    response.headers_mut().insert(
        axum::http::header::STRICT_TRANSPORT_SECURITY,
        axum::http::HeaderValue::from_static(HSTS_VALUE),
    );
    response
}

/// axum middleware that tags every request with the [`super::TlsConn`] marker.
///
/// Layered **only** on the TLS listener's Router clone (never on the plain-HTTP
/// Router), so a downstream handler can prove a request arrived over TLS by
/// extracting `Extension<TlsConn>`. This is the gate that lets the login
/// handler accept a plaintext password over HTTPS while continuing to reject it
/// over plain HTTP.
pub async fn tls_marker_layer(
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    request.extensions_mut().insert(super::TlsConn);
    next.run(request).await
}

/// A TLS-terminating listener that implements [`axum::serve::Listener`].
///
/// `accept()` performs the TCP accept *and* the rustls handshake, reading the
/// live `ServerConfig` from the [`TlsCertStore`] each time so cert rotation
/// takes effect on the next connection. Handshake failures are logged and
/// skipped — they never propagate out of `accept()`, matching the trait
/// contract that the accept future only resolves on a usable connection.
pub struct TlsListener {
    tcp: TcpListener,
    store: TlsCertStore,
}

impl TlsListener {
    /// Bind the TCP socket for the TLS listener. The rustls handshake happens
    /// per-connection in [`axum::serve::Listener::accept`].
    pub async fn bind(addr: SocketAddr, store: TlsCertStore) -> io::Result<Self> {
        let tcp = TcpListener::bind(addr).await?;
        Ok(Self { tcp, store })
    }
}

impl axum::serve::Listener for TlsListener {
    type Io = TlsStream<tokio::net::TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            // 1. Accept the raw TCP connection. Mirror axum's own retry-on-
            //    accept-error behaviour so a transient EMFILE etc. doesn't kill
            //    the listener.
            let (tcp_stream, peer) = match self.tcp.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!(error = %e, "auth TLS accept error; retrying");
                    // Brief backoff on fd exhaustion / transient errors so we
                    // don't spin hot.
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    continue;
                }
            };

            // 2. Read the *current* server config per-connection so a
            //    `reload()` between connections rotates the cert with no
            //    listener restart.
            let acceptor = TlsAcceptor::from(self.store.current());

            // 3. Drive the rustls handshake. A client speaking plain HTTP to
            //    the HTTPS port lands here; we log at debug and move on rather
            //    than tearing down the listener.
            match acceptor.accept(tcp_stream).await {
                Ok(tls_stream) => return (tls_stream, peer),
                Err(e) => {
                    tracing::debug!(peer = %peer, error = %e, "auth TLS handshake failed; dropping connection");
                    continue;
                }
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.tcp.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RAII temp dir under the OS temp root — avoids pulling in the `tempfile`
    /// crate just for these unit tests. Deletes its tree on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let mut p = std::env::temp_dir();
            // Process id + a monotonic counter keeps concurrent test runs from
            // colliding on the same directory.
            use std::sync::atomic::{AtomicU64, Ordering};
            static SEQ: AtomicU64 = AtomicU64::new(0);
            let n = SEQ.fetch_add(1, Ordering::Relaxed);
            p.push(format!("cimmeria-tls-{tag}-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).expect("create temp dir");
            TempDir(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Write a throwaway self-signed cert/key pair to `dir`, returning their
    /// paths. Exercises the production load path end to end (PEM read →
    /// rustls-pemfile parse → ServerConfig build) without a live socket.
    fn write_self_signed(dir: &TempDir) -> (PathBuf, PathBuf) {
        let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .expect("self-signed cert");
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        std::fs::write(&cert_path, certified.cert.pem()).unwrap();
        std::fs::write(&key_path, certified.key_pair.serialize_pem()).unwrap();
        (cert_path, key_path)
    }

    #[test]
    fn load_builds_config_from_pem() {
        let dir = TempDir::new("load");
        let (cert_path, key_path) = write_self_signed(&dir);
        let store = TlsCertStore::load(&cert_path, &key_path).expect("store loads");
        // current() must yield a usable Arc — confirms with_single_cert accepted
        // the rcgen-emitted pair.
        let _cfg = store.current();
    }

    #[test]
    fn reload_swaps_in_place() {
        let dir = TempDir::new("reload");
        let (cert_path, key_path) = write_self_signed(&dir);
        let store = TlsCertStore::load(&cert_path, &key_path).expect("store loads");
        let before = Arc::as_ptr(&store.current());

        // Overwrite the cert files with a fresh pair, then reload.
        let fresh = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
        std::fs::write(&cert_path, fresh.cert.pem()).unwrap();
        std::fs::write(&key_path, fresh.key_pair.serialize_pem()).unwrap();
        store.reload().expect("reload succeeds");

        let after = Arc::as_ptr(&store.current());
        assert_ne!(before, after, "reload must swap in a new ServerConfig Arc");
    }

    #[test]
    fn missing_cert_file_errors() {
        let err = TlsCertStore::load("does-not-exist.pem", "also-missing.pem")
            .expect_err("missing files must error");
        assert!(matches!(err, TlsError::Io { .. }));
    }

    #[tokio::test]
    async fn hsts_layer_stamps_header_on_every_response() {
        use axum::routing::get;
        use axum::Router;
        use tower::ServiceExt; // for `oneshot`

        // A trivial router with the HSTS layer applied — exactly how
        // service.rs wires it onto the auth Router.
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(hsts_layer));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request");

        let hsts = response
            .headers()
            .get(axum::http::header::STRICT_TRANSPORT_SECURITY)
            .expect("HSTS header must be present on every response");
        assert_eq!(hsts.to_str().unwrap(), HSTS_VALUE);
        // Pin the exact policy: two-year max-age + includeSubDomains.
        assert_eq!(HSTS_VALUE, "max-age=63072000; includeSubDomains");
    }
}
