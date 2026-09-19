//! Regression guard: login credentials must never reach a log sink in
//! full.
//!
//! Drives Phase 1 + Phase 2 through the real HTTP stack (same harness as
//! `login_smoke`) under a [`LogCapture`], then asserts that no captured
//! event — at any level, TRACE included — carries the SID, ticket,
//! session key, or password hash the login produced. Reverting any one
//! of the redactions in `handlers.rs` (full `sid`, full `ticket`, raw
//! Phase 1 SOAP body) trips it.
//!
//! The `sid_prefix` / `ticket_prefix` assertions are the positive
//! control: they prove the handler's DEBUG events were captured at all,
//! so a pass can't come from the capture silently seeing nothing.

use cimmeria_common::ServerConfig;

use super::login_smoke::{extract_attr, start_auth_on_ephemeral_port};
use super::ShardInfo;
use crate::credential_redaction::CredentialPrefix;
use crate::test_support::LogCapture;

const PASSWORD_HASH: &str = "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3";

// Default `#[tokio::test]` is current-thread, so the spawned axum
// handlers run on this thread and `LogCapture` sees their events.
#[tokio::test]
async fn phase1_and_phase2_never_log_full_credentials() {
    let capture = LogCapture::install();

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
    let (mut auth, port) = start_auth_on_ephemeral_port(&base_config, &shards).await;
    let base_url = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::new();

    let phase1_body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="leak-guard-user" Password="{PASSWORD_HASH}" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#
    );
    let resp1 = client
        .post(format!("{base_url}/SGWLogin/UserAuth"))
        .header("Content-Type", "text/xml")
        .body(phase1_body)
        .send()
        .await
        .expect("Phase 1 POST");
    let sid = resp1
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(';').find_map(|p| p.trim().strip_prefix("SID=")))
        .map(str::to_string)
        .expect("Phase 1 response must carry a SID cookie");

    let phase2_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" ServerSelection="TestShard" />"#;
    let phase2_xml = client
        .post(format!("{base_url}/SGWLogin/ServerSelection"))
        .header("Content-Type", "text/xml")
        .header("Cookie", format!("SID={sid}"))
        .body(phase2_body)
        .send()
        .await
        .expect("Phase 2 POST")
        .text()
        .await
        .expect("Phase 2 body");
    let session_key = extract_attr(&phase2_xml, "SessionKey").expect("SessionKey attribute");
    let ticket = extract_attr(&phase2_xml, "Ticket").expect("Ticket attribute");

    auth.stop().await;

    let events = capture.all();

    // Positive control — the handlers' DEBUG events were captured, and
    // they carry the redacted prefix rather than nothing at all.
    let has_field = |name: &str, value: &str| {
        events
            .iter()
            .any(|e| e.fields.get(name).is_some_and(|v| v == value))
    };
    assert!(
        has_field("sid_prefix", &CredentialPrefix(&sid).to_string()),
        "Phase 1 must log the SID as a redacted `sid_prefix` field"
    );
    assert!(
        has_field("ticket_prefix", &CredentialPrefix(&ticket).to_string()),
        "Phase 2 must log the ticket as a redacted `ticket_prefix` field"
    );

    for (label, secret) in [
        ("SID", sid.as_str()),
        ("ticket", ticket.as_str()),
        ("session key", session_key.as_str()),
        ("password hash", PASSWORD_HASH),
    ] {
        for event in &events {
            let leaked = event.message.as_deref().is_some_and(|m| m.contains(secret))
                || event.fields.values().any(|v| v.contains(secret));
            assert!(
                !leaked,
                "full {label} leaked into a {} event on target `{}`: {event:?}",
                event.level, event.target
            );
        }
    }
}
