//! Login smoke: full HTTP-stack drive of Phase 1 (UserAuth) +
//! Phase 2 (ServerSelection) against a real `AuthService` started
//! on a kernel-assigned localhost port.
//!
//! What this catches that the per-handler tests don't:
//!
//! - Axum routing drift: `/SGWLogin/UserAuth` and
//!   `/SGWLogin/ServerSelection` must remain mounted on the auth
//!   `Router`. A regression that renamed a route or moved it under
//!   a prefix would surface here as a 404, not in the parser tests.
//! - Real cookie round-trip: Phase 1 emits `SET_COOKIE: SID=...`,
//!   Phase 2 requires it back as `Cookie: SID=...`. The per-handler
//!   tests don't exercise the HTTP cookie machinery; this smoke does.
//! - Phase-1-to-Phase-2 session continuity: the SID is an in-memory
//!   handoff between the two requests. A regression that broke the
//!   `state.sessions` map (e.g., switched to a per-request store)
//!   would still pass per-handler tests but fail the smoke at Phase 2.
//! - Shard-selection round-trip on the live response: the Phase 2
//!   response carries the SessionKey + Ticket attributes that Phase 3
//!   consumes; a drift in the XML shape would catch here.
//!
//! Runs in `developer_mode` so no DB is required — the credential
//! check is short-circuited to "any well-formed request succeeds".
//! The DB-credential path is exercised by the per-handler unit tests
//! in `handlers.rs`.

use std::net::TcpListener as StdTcpListener;

use cimmeria_common::ServerConfig;

use super::{AuthService, ShardInfo};

/// Pick a fresh ephemeral port by binding and immediately dropping a
/// `127.0.0.1:0` listener — the kernel returns the next free port on
/// the std listener and `AuthService::start` then binds that port via
/// tokio. There is a TOCTOU window between drop and rebind where a
/// concurrent test or process could grab the port; the retry loop in
/// [`start_auth_on_ephemeral_port`] absorbs that race.
fn ephemeral_port() -> u16 {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind ephemeral TCP listener");
    let port = listener.local_addr().expect("local_addr").port();
    drop(listener);
    port
}

/// Construct + start an `AuthService` on a fresh ephemeral port,
/// retrying with a different port if the bind races a concurrent
/// claim. Caps at 5 attempts so a genuine bind failure (broken
/// permissions, OS exhaustion) doesn't hang the test forever.
///
/// Returns the running service and the port it bound to. Closes the
/// TOCTOU loophole the previous "bind once, hope nobody steals it"
/// path had.
async fn start_auth_on_ephemeral_port(
    base_config: &ServerConfig,
    shards: &[ShardInfo],
) -> (AuthService, u16) {
    const MAX_ATTEMPTS: usize = 5;
    let mut last_err: Option<String> = None;
    for attempt in 1..=MAX_ATTEMPTS {
        let port = ephemeral_port();
        let mut config = base_config.clone();
        config.logon_port = port;
        let mut auth = AuthService::new(&config);
        for shard in shards {
            auth.register_shard(shard.clone());
        }
        match auth.start().await {
            Ok(()) => return (auth, port),
            Err(e) => {
                tracing::debug!(
                    attempt,
                    port,
                    error = %e,
                    "auth bind raced; retrying with a fresh port"
                );
                last_err = Some(e.to_string());
            }
        }
    }
    panic!(
        "failed to start AuthService on an ephemeral port after {MAX_ATTEMPTS} attempts (last error: {last_err:?})"
    );
}

#[tokio::test]
async fn login_smoke_drives_phase1_and_phase2_through_real_http_stack() {
    // ── 1. Configure + start auth on an ephemeral local port ────────
    // developer_mode short-circuits the DB credential check so this
    // smoke runs without DATABASE_URL. The DB-credential path is
    // covered by the per-handler tests in handlers.rs.
    let base_config = ServerConfig {
        auth_host: "127.0.0.1".to_string(),
        developer_mode: true,
        ..ServerConfig::default()
    };
    let shards = vec![ShardInfo {
        name: "TestShard".to_string(),
        host: "127.0.0.1".to_string(),
        port: 32832,
        protected: false,
    }];
    // AuthService::start awaits TcpListener::bind before returning,
    // so once start_auth_on_ephemeral_port returns Ok the listener
    // is already accepting connections — no readiness probe needed.
    let (mut auth, port) = start_auth_on_ephemeral_port(&base_config, &shards).await;
    let base_url = format!("http://127.0.0.1:{port}");

    // Default reqwest::Client has no cookie store. We explicitly
    // forward the SID below so a regression that breaks the cookie
    // round-trip surfaces here rather than being papered over by an
    // automatic jar.
    let client = reqwest::Client::new();

    // ── 3. Phase 1: POST /SGWLogin/UserAuth ─────────────────────────
    // Raw-string body — backslashes are literal in `r#"..."#` and
    // would land in the wire bytes, so this is plain XML on one
    // logical line per element. Newlines inside attribute values are
    // tolerated by quick-xml but not added here to keep the wire
    // payload close to what the real client sends.
    let phase1_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="smoke-user" Password="A94A8FE5CCB19BA61C4C0873D391E987982FBBD3" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;
    let resp1 = client
        .post(format!("{base_url}/SGWLogin/UserAuth"))
        .header("Content-Type", "text/xml")
        .body(phase1_body)
        .send()
        .await
        .expect("Phase 1 POST must succeed");
    assert_eq!(
        resp1.status(),
        reqwest::StatusCode::OK,
        "Phase 1 must return 200"
    );
    // Pull the SID out of the Set-Cookie header before consuming the
    // body — we need it for Phase 2.
    let sid = resp1
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';').find_map(|p| {
                let p = p.trim();
                p.strip_prefix("SID=").map(str::to_string)
            })
        })
        .expect("Phase 1 response must carry a SID cookie");
    assert!(!sid.is_empty(), "SID must be non-empty");
    assert_eq!(
        sid.len(),
        40,
        "SID matches the C++ session-cookie format (40 chars)"
    );
    // Charset pin: the C++ format is alphanumeric. Length-only check
    // would still pass if the SID regressed to include punctuation,
    // newlines, or quoted bytes — any of which break the
    // `Cookie: SID=...` round-trip on the client side.
    assert!(
        sid.chars().all(|c| c.is_ascii_alphanumeric()),
        "SID must be ASCII alphanumeric (got {sid:?})"
    );

    let phase1_xml = resp1.text().await.expect("Phase 1 body");
    assert!(
        phase1_xml.contains("ns2:SGWLoginResponse") && phase1_xml.contains("SGWLoginSuccess"),
        "Phase 1 XML must contain a success envelope, got: {phase1_xml}"
    );
    assert!(
        phase1_xml.contains(r#"ServerName="TestShard""#),
        "Phase 1 must advertise the registered shard"
    );

    // ── 4. Phase 2: POST /SGWLogin/ServerSelection with Cookie ──────
    let phase2_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" ServerSelection="TestShard" />"#;
    let resp2 = client
        .post(format!("{base_url}/SGWLogin/ServerSelection"))
        .header("Content-Type", "text/xml")
        .header("Cookie", format!("SID={sid}"))
        .body(phase2_body)
        .send()
        .await
        .expect("Phase 2 POST must succeed");
    assert_eq!(
        resp2.status(),
        reqwest::StatusCode::OK,
        "Phase 2 must return 200"
    );

    let phase2_xml = resp2.text().await.expect("Phase 2 body");
    assert!(
        phase2_xml.contains("ns3:SGWServerLocationResponse"),
        "Phase 2 XML must be a ServerLocationResponse, got: {phase2_xml}"
    );

    // Pin both load-bearing attributes from the Phase 2 response —
    // these are what the client uses to drive the Phase 3 UDP
    // baseAppLogin packet.
    let session_key = extract_attr(&phase2_xml, "SessionKey")
        .expect("Phase 2 response must carry SessionKey attribute");
    assert_eq!(
        session_key.len(),
        64,
        "session_key matches the 64-char hex format expected by decode_session_key"
    );
    assert!(
        session_key.chars().all(|c| c.is_ascii_hexdigit()),
        "session_key must be hex"
    );

    let ticket = extract_attr(&phase2_xml, "Ticket").expect("Phase 2 must carry Ticket attribute");
    assert_eq!(
        ticket.len(),
        20,
        "ticket matches the 20-byte format expected by parse_baseapp_login"
    );

    // ── 5. Replay Phase 2 with the same SID — must fail ─────────────
    // SIDs are single-use. A regression that left the session in the
    // store after Phase 2 would silently allow ticket harvesting.
    let resp3 = client
        .post(format!("{base_url}/SGWLogin/ServerSelection"))
        .header("Content-Type", "text/xml")
        .header("Cookie", format!("SID={sid}"))
        .body(phase2_body)
        .send()
        .await
        .expect("replay POST must succeed at the HTTP level");
    let replay_xml = resp3.text().await.expect("replay body");
    assert!(
        replay_xml.contains("ServerSelectionError"),
        "second use of the same SID must produce a SelectionError (sessions are single-use)"
    );

    // ── 6. Clean shutdown ───────────────────────────────────────────
    auth.stop().await;
    // Note: auth.stop() flips is_running but doesn't actively reap
    // the spawned axum task — that's a known gap in the service
    // shutdown story. We assert the documented contract: stop()
    // returns without error.
}

/// Tiny attribute extractor for `Key="value"` patterns, scoped to this
/// smoke. quick-xml's full reader works but is overkill here — we
/// just need to pull two attributes out of a known-shape success
/// response.
fn extract_attr(xml: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=\"");
    let start = xml.find(&needle)? + needle.len();
    let end = start + xml[start..].find('"')?;
    Some(xml[start..end].to_string())
}
