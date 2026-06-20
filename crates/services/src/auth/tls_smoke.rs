//! Auth-TLS smoke (#434 Phase 1): generate a self-signed cert at runtime, boot
//! the `AuthService` with the TLS listener on an ephemeral port, and drive a
//! Phase-1 login request **over TLS** with a reqwest client that pins the
//! self-signed cert as a trusted root.
//!
//! What this catches that the `tls.rs` unit tests don't:
//!
//! - The full parallel-listener wiring in `AuthService::start`: the TLS
//!   listener must come up *alongside* the HTTP listener and serve the same
//!   Router. A regression that dropped the TLS spawn, mounted the wrong router,
//!   or bound the wrong address surfaces here as a failed handshake or a 404.
//! - The `axum::serve::Listener` impl on `TlsListener` end to end: a real
//!   rustls handshake + HTTP/1.1 request/response over the wrapped stream.
//! - The HSTS header on the live TLS path (spec item 4).
//!
//! Runs in `developer_mode` so no DB is required — the credential check is
//! short-circuited exactly as the plain-HTTP `login_smoke` does.

use std::net::TcpListener as StdTcpListener;
use std::path::PathBuf;

use cimmeria_common::ServerConfig;

use super::{AuthService, ShardInfo};

/// RAII temp dir holding the throwaway cert/key. Deletes its tree on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let mut p = std::env::temp_dir();
        p.push(format!("cimmeria-tls-smoke-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&p).expect("create temp dir");
        TempDir(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Pick a fresh ephemeral port by binding+dropping a `127.0.0.1:0` listener.
/// Same TOCTOU caveat as the HTTP smoke; the start path tolerates a lost race
/// by erroring, and we cap retries below.
fn ephemeral_port() -> u16 {
    let l = StdTcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let p = l.local_addr().expect("local_addr").port();
    drop(l);
    p
}

/// Generate a self-signed cert whose SAN is `localhost`, returning the PEM
/// bytes for (cert, key). We connect to `localhost` (forced to 127.0.0.1 via
/// reqwest's resolver) so the rustls SAN check on the client passes.
fn self_signed_pem() -> (String, String) {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .expect("self-signed cert");
    (certified.cert.pem(), certified.key_pair.serialize_pem())
}

#[tokio::test]
async fn tls_listener_serves_phase1_login_over_https() {
    // ── 1. Write a runtime self-signed cert ─────────────────────────────
    let dir = TempDir::new();
    let (cert_pem, key_pem) = self_signed_pem();
    let cert_path = dir.0.join("cert.pem");
    let key_path = dir.0.join("key.pem");
    std::fs::write(&cert_path, &cert_pem).expect("write cert");
    std::fs::write(&key_path, &key_pem).expect("write key");

    // ── 2. Boot AuthService with TLS configured on an ephemeral port ────
    // developer_mode short-circuits the DB credential check so this runs
    // without DATABASE_URL.
    const MAX_ATTEMPTS: usize = 5;
    let (mut auth, tls_port) = {
        let mut started = None;
        for _ in 0..MAX_ATTEMPTS {
            let http_port = ephemeral_port();
            let tls_port = ephemeral_port();
            let config = ServerConfig {
                auth_host: "127.0.0.1".to_string(),
                logon_port: http_port,
                auth_tls_port: tls_port,
                auth_tls_cert_path: Some(cert_path.clone()),
                auth_tls_key_path: Some(key_path.clone()),
                developer_mode: true,
                ..ServerConfig::default()
            };
            let mut auth = AuthService::new(&config);
            auth.register_shard(ShardInfo {
                name: "TlsShard".to_string(),
                host: "127.0.0.1".to_string(),
                port: 32832,
                protected: false,
            });
            if auth.start().await.is_ok() {
                started = Some((auth, tls_port));
                break;
            }
        }
        started.expect("AuthService failed to start with TLS after retries")
    };

    // ── 3. Build a reqwest client that trusts our self-signed cert ──────
    // The cert SAN is `localhost`; force `localhost` to resolve to the
    // loopback TLS port so the rustls SAN check passes.
    let root = reqwest::Certificate::from_pem(cert_pem.as_bytes()).expect("parse root cert");
    let addr = format!("127.0.0.1:{tls_port}")
        .parse::<std::net::SocketAddr>()
        .unwrap();
    let client = reqwest::Client::builder()
        .add_root_certificate(root)
        .resolve("localhost", addr)
        // No `danger_accept_invalid_certs` — the whole point is that the
        // pinned root validates the handshake.
        .build()
        .expect("build TLS client");

    let base_url = format!("https://localhost:{tls_port}");

    // ── 4. Phase 1 login over HTTPS ─────────────────────────────────────
    let phase1_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="tls-user" Password="A94A8FE5CCB19BA61C4C0873D391E987982FBBD3" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;

    let resp = client
        .post(format!("{base_url}/SGWLogin/UserAuth"))
        .header("Content-Type", "text/xml")
        .body(phase1_body)
        .send()
        .await
        .expect("Phase 1 POST over TLS must succeed");

    assert_eq!(
        resp.status(),
        reqwest::StatusCode::OK,
        "Phase 1 over TLS must return 200"
    );

    // HSTS must be present on the TLS path.
    let hsts = resp
        .headers()
        .get(reqwest::header::STRICT_TRANSPORT_SECURITY)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    assert_eq!(
        hsts.as_deref(),
        Some(super::tls::HSTS_VALUE),
        "HSTS header must be stamped on the TLS response"
    );

    let xml = resp.text().await.expect("Phase 1 body");
    assert!(
        xml.contains("ns2:SGWLoginResponse") && xml.contains("SGWLoginSuccess"),
        "Phase 1 over TLS must return a success envelope, got: {xml}"
    );
    assert!(
        xml.contains(r#"ServerName="TlsShard""#),
        "Phase 1 over TLS must advertise the registered shard"
    );

    auth.stop().await;
}
