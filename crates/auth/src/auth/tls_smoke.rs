//! Auth-TLS smoke: generate a self-signed cert at runtime, boot
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
//! - XML-escaped plaintext passwords over TLS (#1289): an escaped password
//!   verifies against the decoded one (live DB), and malformed entity syntax
//!   is refused with a logged reason that never quotes the password.
//!
//! The first two tests run in `developer_mode` so no DB is required — the
//! credential check is short-circuited exactly as the plain-HTTP
//! `login_smoke` does. The escaped-password test needs the real credential
//! check, so it is live-DB.

use std::net::TcpListener as StdTcpListener;
use std::path::PathBuf;
use std::sync::Arc;

use cimmeria_common::ServerConfig;
use sqlx::PgPool;

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
    (certified.cert.pem(), certified.signing_key.serialize_pem())
}

/// A running `AuthService` with its TLS listener up, plus a reqwest client
/// that trusts the throwaway cert. Holds the cert dir so it outlives the
/// service.
struct TlsAuth {
    auth: AuthService,
    client: reqwest::Client,
    base_url: String,
    _dir: TempDir,
}

/// Boot `AuthService` with TLS on an ephemeral port. `db` = `None` runs in
/// `developer_mode` (no credential check); `Some` runs the real check.
async fn start_tls_auth(shard_name: &str, db: Option<Arc<PgPool>>) -> TlsAuth {
    // ── Write a runtime self-signed cert ────────────────────────────────
    let dir = TempDir::new();
    let (cert_pem, key_pem) = self_signed_pem();
    let cert_path = dir.0.join("cert.pem");
    let key_path = dir.0.join("key.pem");
    std::fs::write(&cert_path, &cert_pem).expect("write cert");
    std::fs::write(&key_path, &key_pem).expect("write key");

    // ── Boot AuthService with TLS configured on an ephemeral port ───────
    const MAX_ATTEMPTS: usize = 5;
    let (auth, tls_port) = {
        let mut started = None;
        for _ in 0..MAX_ATTEMPTS {
            let http_port = ephemeral_port();
            let tls_port = ephemeral_port();
            let config = ServerConfig {
                logon_port: http_port,
                auth_tls_port: tls_port,
                auth_tls_cert_path: Some(cert_path.clone()),
                auth_tls_key_path: Some(key_path.clone()),
                developer_mode: db.is_none(),
                ..ServerConfig::loopback()
            };
            let mut auth = AuthService::new(&config);
            if let Some(pool) = &db {
                auth.set_db_pool(Arc::clone(pool));
            }
            auth.register_shard(ShardInfo {
                name: shard_name.to_string(),
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

    // ── A reqwest client that trusts our self-signed cert ───────────────
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

    TlsAuth {
        auth,
        client,
        base_url: format!("https://localhost:{tls_port}"),
        _dir: dir,
    }
}

/// A Phase 1 body whose `Password` attribute is `raw_password`, spelled
/// exactly as it sits between the quotes on the wire.
fn phase1_body(account: &str, raw_password: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="{account}" Password="{raw_password}" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#
    )
}

/// POST a Phase 1 body over TLS and return the response XML.
async fn post_phase1(tls: &TlsAuth, body: String) -> String {
    let resp = tls
        .client
        .post(format!("{}/SGWLogin/UserAuth", tls.base_url))
        .header("Content-Type", "text/xml")
        .body(body)
        .send()
        .await
        .expect("Phase 1 POST over TLS must succeed");
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::OK,
        "auth always returns 200"
    );
    resp.text().await.expect("Phase 1 body")
}

#[tokio::test]
async fn tls_listener_serves_phase1_login_over_https() {
    let mut tls = start_tls_auth("TlsShard", None).await;

    let resp = tls
        .client
        .post(format!("{}/SGWLogin/UserAuth", tls.base_url))
        .header("Content-Type", "text/xml")
        .body(phase1_body(
            "tls-user",
            "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3",
        ))
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

    tls.auth.stop().await;
}

/// **#1289 malformed-entity guard.** A plaintext password with an unknown
/// entity (`&SecretFragment;`) over TLS must be refused, and the refusal must
/// log `reason = "unrecognized_entity"` / `attribute = "Password"` without
/// quoting any part of the password. Runs in `developer_mode`, where any
/// password that reaches the credential check succeeds — so with the decode
/// reverted (raw value passed through) this login *succeeds* and the guard
/// trips.
// Default `#[tokio::test]` is current-thread, so the spawned axum handlers
// run on this thread and `LogCapture` sees their events.
#[tokio::test]
async fn tls_phase1_malformed_password_entity_is_rejected_and_logged() {
    let capture = crate::test_support::LogCapture::install();
    let mut tls = start_tls_auth("TlsShard", None).await;

    let xml = post_phase1(&tls, phase1_body("tls-user", "pw&SecretFragment;x")).await;
    tls.auth.stop().await;

    assert!(
        xml.contains("SGWLoginError") && !xml.contains("SGWLoginSuccess"),
        "a malformed entity in Password must fail the login, got: {xml}"
    );

    let warn = capture
        .find_event(
            tracing::Level::WARN,
            "Phase 1 SOAP request rejected",
            "unrecognized_entity",
        )
        .expect("malformed Password entity must log reason=unrecognized_entity at WARN");
    assert_eq!(
        warn.fields.get("attribute").map(String::as_str),
        Some("Password")
    );

    for event in capture.all() {
        let leaked = event
            .message
            .as_deref()
            .is_some_and(|m| m.contains("SecretFragment"))
            || event.fields.values().any(|v| v.contains("SecretFragment"));
        assert!(
            !leaked,
            "password fragment leaked into a log event: {event:?}"
        );
    }
}

/// Sentinel account id for the escaped-password smoke. Inside the crate's
/// `0x7000_1B00` credential window, past the slots `credentials.rs` uses
/// (`+1`..`+7`) and clear of `audit.rs`'s `0x7000_1B40`.
const ESCAPED_PW_ACCOUNT_ID: i32 = 0x7000_1B20;

async fn delete_account(pool: &PgPool, account_id: i32) {
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// **#1289 regression guard (live DB, real TLS).** An argon2id account whose
/// password contains `&`, `<` and `"` logs in over TLS when the client
/// escapes those characters in the SOAP attribute, as XML requires. Before
/// the fix the server compared the escaped wire spelling
/// (`a&amp;b&lt;c&quot;d`) against the stored hash of `a&b<c"d` and returned
/// "The account name or password is incorrect."
#[tokio::test]
async fn live_db_tls_phase1_accepts_xml_escaped_plaintext_password() {
    use crate::test_support::require_db_or_skip;
    let pool = require_db_or_skip!();

    const PASSWORD: &str = r#"a&b<c"d"#;
    const WIRE_PASSWORD: &str = "a&amp;b&lt;c&quot;d";
    let name = "tlsesc1b20";

    delete_account(&pool, ESCAPED_PW_ACCOUNT_ID).await;
    let phc = super::password_hash::hash_argon2id(PASSWORD).expect("hash fixture password");
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_hash_v2, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, NULL, $3, 2, 0, true)",
    )
    .bind(ESCAPED_PW_ACCOUNT_ID)
    .bind(name)
    .bind(phc)
    .execute(&pool)
    .await
    .expect("insert argon2id fixture account");

    let mut tls = start_tls_auth("TlsShard", Some(Arc::new(pool.clone()))).await;
    let ok_xml = post_phase1(&tls, phase1_body(name, WIRE_PASSWORD)).await;
    // Positive control for the comparison itself: the raw (wrong) password
    // spelled correctly still fails, so the success above is the decode, not
    // a credential check that accepts anything.
    let wrong_xml = post_phase1(&tls, phase1_body(name, "a&amp;b&lt;c&quot;X")).await;
    tls.auth.stop().await;
    delete_account(&pool, ESCAPED_PW_ACCOUNT_ID).await;

    assert!(
        ok_xml.contains("SGWLoginSuccess"),
        "escaped plaintext password must verify against the decoded password, got: {ok_xml}"
    );
    assert!(
        ok_xml.contains(&format!(r#"AccountId="{ESCAPED_PW_ACCOUNT_ID}""#)),
        "success must be for the fixture account, got: {ok_xml}"
    );
    assert!(
        wrong_xml.contains("SGWLoginError"),
        "a wrong password must still fail, got: {wrong_xml}"
    );
}
