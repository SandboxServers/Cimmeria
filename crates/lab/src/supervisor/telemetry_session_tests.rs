//! The lab's dev-session mint: the request it sends, the URL it sends it
//! to, and what it writes from the answer, including against a real HTTP
//! listener standing in for the server.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;

#[test]
fn dev_session_url_accepts_a_base_with_or_without_api() {
    for base in [
        "http://127.0.0.1:8443",
        "http://127.0.0.1:8443/",
        "http://127.0.0.1:8443/api",
        "http://127.0.0.1:8443/api/",
    ] {
        assert_eq!(
            dev_session_url(base),
            "http://127.0.0.1:8443/api/auth/dev-session",
            "{base}"
        );
    }
}

/// The body the server's `DevSessionRequest` parses, with the lab kind
/// that tags every uploaded row. An `install_id` outside the server's
/// alphabet (ASCII alphanumeric, `-`, `_`) would be a 400.
#[test]
fn the_mint_request_asks_for_a_lab_session() {
    let v = serde_json::to_value(MintRequest::lab()).unwrap();
    assert_eq!(v["session_kind"], "lab");
    assert_eq!(v["install_id"], "cimmeria-lab");
    assert!(LAB_INSTALL_ID
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
    for field in [
        "machine_id",
        "branch",
        "git_sha",
        "launcher_version",
        "tags",
    ] {
        assert!(v.get(field).is_some(), "{field} is required by the server");
    }
}

fn response() -> MintResponse {
    MintResponse {
        session_id: "sid".into(),
        token: "tok".into(),
        expires_at_ms: 5,
        upload_endpoint: "https://colo.example/api/telemetry".into(),
        chunk_max_bytes: 7,
        flush_interval_ms: 9,
    }
}

#[test]
fn a_minted_grant_uses_the_server_endpoint_unless_overridden() {
    let g = TelemetryGrant::minted(response(), &TelemetryConfig::default());
    assert_eq!(g.upload_endpoint, "https://colo.example/api/telemetry");
    assert_eq!(g.token, "tok");
    assert_eq!(g.status()["enabled"], true);

    let cfg = TelemetryConfig {
        upload_endpoint_override: Some("http://10.0.0.2:8443/api/telemetry".into()),
        ..TelemetryConfig::default()
    };
    let g = TelemetryGrant::minted(response(), &cfg);
    assert_eq!(g.upload_endpoint, "http://10.0.0.2:8443/api/telemetry");
}

#[test]
fn an_unavailable_grant_says_why_and_points_at_the_configured_server() {
    let cfg = TelemetryConfig {
        server_url: "http://192.0.2.1:8443/api".into(),
        upload_endpoint_override: None,
    };
    let g = TelemetryGrant::unavailable("refused".into(), &cfg);
    assert!(g.token.is_empty());
    assert_eq!(g.upload_endpoint, "http://192.0.2.1:8443/api/telemetry");
    assert_eq!(g.status()["enabled"], false);
    assert_eq!(g.status()["reason"], "refused");
}

/// Serve one HTTP request on a loopback listener: return the request text
/// and answer with `status` and `body`.
async fn serve_once(
    status: &'static str,
    body: &'static str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut req = Vec::new();
        let mut buf = [0u8; 4096];
        // Read until the JSON body's closing brace; the request is small.
        loop {
            let n = sock.read(&mut buf).await.unwrap();
            req.extend_from_slice(&buf[..n]);
            if n == 0 || req.ends_with(b"}") {
                break;
            }
        }
        let resp = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        sock.write_all(resp.as_bytes()).await.unwrap();
        String::from_utf8_lossy(&req).into_owned()
    });
    (base, handle)
}

/// **The lab flow end to end, minus the game.** The supervisor POSTs the
/// lab request to `/api/auth/dev-session` on the configured server and
/// writes the token and endpoint the server returns.
#[tokio::test]
async fn grant_for_launch_mints_from_the_configured_server() {
    let (base, server) = serve_once(
        "200 OK",
        r#"{"session_id":"sid-1","token":"p.s","expires_at_ms":42,"upload_endpoint":"http://127.0.0.1:8443/api/telemetry","chunk_max_bytes":1048576,"flush_interval_ms":2000}"#,
    )
    .await;
    let cfg = TelemetryConfig {
        server_url: base,
        upload_endpoint_override: None,
    };
    let g = grant_for_launch(&cfg).await;
    let req = server.await.unwrap();

    assert!(req.starts_with("POST /api/auth/dev-session "), "{req}");
    assert!(req.contains(r#""session_kind":"lab""#), "{req}");
    assert_eq!(g.unavailable, None);
    assert_eq!(g.token, "p.s");
    assert_eq!(g.session_id, "sid-1");
    assert_eq!(g.upload_endpoint, "http://127.0.0.1:8443/api/telemetry");
}

/// A server that refuses (no `CIMMERIA_TELEMETRY_HMAC_SECRET`: 500) gives
/// an unavailable grant carrying the server's message, never a panic or a
/// stopped launch.
#[tokio::test]
async fn a_refused_mint_is_reported_not_fatal() {
    let (base, server) = serve_once(
        "500 Internal Server Error",
        "Telemetry auth misconfigured: CIMMERIA_TELEMETRY_HMAC_SECRET not set.",
    )
    .await;
    let cfg = TelemetryConfig {
        server_url: base,
        upload_endpoint_override: None,
    };
    let g = grant_for_launch(&cfg).await;
    server.await.unwrap();
    let why = g.unavailable.expect("refused mint");
    assert!(why.contains("500"), "{why}");
    assert!(why.contains("HMAC_SECRET"), "{why}");
    assert!(g.token.is_empty());
}

fn sample_mint(expires_at_ms: i64) -> MintResponse {
    MintResponse {
        session_id: "sid-cached".into(),
        token: "cached.tok".into(),
        expires_at_ms,
        upload_endpoint: "http://127.0.0.1:8443/api/telemetry".into(),
        chunk_max_bytes: 1 << 20,
        flush_interval_ms: 2000,
    }
}

/// **Relaunches reuse the token.** A cached mint for the same server with
/// hours left is used without contacting the server at all (the config
/// points at a closed port, so a mint attempt would fail).
#[tokio::test]
async fn a_valid_cached_token_is_reused_without_minting() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("lab-telemetry-grant.json");
    let cfg = TelemetryConfig {
        server_url: "http://127.0.0.1:9".into(),
        upload_endpoint_override: None,
    };
    let cached = CachedMint {
        server_url: cfg.server_url.clone(),
        mint: sample_mint(now_ms() + 4 * 3600 * 1000),
    };
    std::fs::write(&cache, serde_json::to_vec(&cached).unwrap()).unwrap();
    let g = grant_for_launch_cached(&cfg, &cache).await;
    assert_eq!(g.unavailable, None);
    assert_eq!(g.token, "cached.tok");
    assert_eq!(g.session_id, "sid-cached");
}

/// A token about to expire, or minted by another server, is not reused.
#[test]
fn a_stale_or_foreign_cached_token_is_not_reused() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("g.json");
    let cfg = TelemetryConfig {
        server_url: "http://a:8443".into(),
        upload_endpoint_override: None,
    };
    let now = 1_000_000_000;
    let write = |server: &str, exp: i64| {
        let c = CachedMint {
            server_url: server.into(),
            mint: sample_mint(exp),
        };
        std::fs::write(&cache, serde_json::to_vec(&c).unwrap()).unwrap();
    };
    write("http://a:8443", now + REUSE_MIN_REMAINING_MS - 1);
    assert_eq!(cached_mint(&cache, &cfg, now), None);
    write("http://b:8443", now + 4 * 3600 * 1000);
    assert_eq!(cached_mint(&cache, &cfg, now), None);
    write("http://a:8443", now + REUSE_MIN_REMAINING_MS);
    assert!(cached_mint(&cache, &cfg, now).is_some());
}

/// A fresh mint is written to the cache for the next launch.
#[tokio::test]
async fn a_fresh_mint_is_cached_for_the_next_launch() {
    let (base, server) = serve_once(
        "200 OK",
        r#"{"session_id":"sid-2","token":"n.t","expires_at_ms":99999999999999,"upload_endpoint":"http://127.0.0.1:8443/api/telemetry","chunk_max_bytes":1048576,"flush_interval_ms":2000}"#,
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("lab-telemetry-grant.json");
    let cfg = TelemetryConfig {
        server_url: base,
        upload_endpoint_override: None,
    };
    let g = grant_for_launch_cached(&cfg, &cache).await;
    server.await.unwrap();
    assert_eq!(g.token, "n.t");
    assert_eq!(
        cached_mint(&cache, &cfg, now_ms()).map(|m| m.session_id),
        Some("sid-2".into())
    );
}
