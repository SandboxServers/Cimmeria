use super::*;
use crate::client_setup::login_servers::LoginServer;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn identity() -> Identity {
    Identity {
        install_id: Uuid::from_u128(7),
        machine_id: "0123456789abcdef".into(),
    }
}
fn servers(url: &str) -> Vec<LoginServer> {
    vec![LoginServer {
        name: "fixture".into(),
        url: url.into(),
    }]
}
fn reply(upload_endpoint: &str) -> serde_json::Value {
    serde_json::json!({
        "session_id": "session-1", "token": "payload.sig", "expires_at_ms": 1_700_028_800_000i64,
        "upload_endpoint": upload_endpoint, "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
    })
}

#[test]
fn the_choice_is_off_until_made_and_is_not_the_launcher_summary_consent() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    assert_eq!(state.game_telemetry().unwrap(), GameTelemetry::default());
    // Turning launcher summaries on leaves game telemetry off, with no identity.
    state.save_preferences(None, true, 0).unwrap();
    let before = state.game_telemetry().unwrap();
    assert!(!before.opted_in && before.identity.is_none());

    let on = state.set_game_telemetry(true).unwrap();
    let minted = on
        .identity
        .clone()
        .expect("the first opt-in mints an identity");
    assert!(on.opted_in && minted.valid());
    // Turning game telemetry on did not touch the other consent, and neither
    // record carries the other's field.
    assert!(state.preferences().launcher_summary_consent);
    assert!(
        !std::fs::read_to_string(root.path().join("preferences.json"))
            .unwrap()
            .contains("opted_in")
    );

    // Opting out keeps the identity; opting back in does not mint another.
    let off = state.set_game_telemetry(false).unwrap();
    assert!(!off.opted_in && off.identity == Some(minted.clone()));
    drop(state);
    let mut state = DesktopState::open(root.path()).unwrap();
    assert_eq!(state.game_telemetry().unwrap(), off);
    assert_eq!(
        state.set_game_telemetry(true).unwrap().identity,
        Some(minted)
    );
}

#[test]
fn a_record_that_claims_consent_without_an_identity_is_refused_not_repaired() {
    let root = tempfile::tempdir().unwrap();
    let state = DesktopState::open(root.path()).unwrap();
    std::fs::write(
        state.state_root().join(NAME),
        br#"{"schema_version":1,"opted_in":true,"identity":null}"#,
    )
    .unwrap();
    assert_eq!(state.game_telemetry(), Err(StorageError::Corrupt));
    std::fs::write(
        state.state_root().join(NAME),
        br#"{"schema_version":2,"opted_in":false,"identity":null}"#,
    )
    .unwrap();
    assert_eq!(state.game_telemetry(), Err(StorageError::UnsupportedSchema));
}

#[tokio::test]
async fn a_minted_session_carries_the_identity_tags_and_the_servers_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .and(body_partial_json(serde_json::json!({
            "install_id": Uuid::from_u128(7).to_string(),
            "machine_id": "0123456789abcdef",
            "tags": ["desktop-launcher", "macos", "wine"],
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(reply(&format!("{}/api/telemetry", server.uri()))),
        )
        .expect(1)
        .mount(&server)
        .await;
    let session = start(
        &identity(),
        &servers(&server.uri()),
        &["desktop-launcher", "macos", "wine"],
    )
    .await
    .unwrap();
    let marker = session.marker();
    assert_eq!(marker.session_id, "session-1");
    assert_eq!(marker.install_id, Uuid::from_u128(7).to_string());
    assert_eq!(marker.machine_id, "0123456789abcdef");
    assert!(marker.telemetry.enabled);
    assert_eq!(marker.telemetry.token, "payload.sig");
    assert_eq!(
        marker.telemetry.upload_endpoint,
        format!("{}/api/telemetry", server.uri())
    );
    assert_eq!(marker.tags, ["desktop-launcher", "macos", "wine"]);
    // The DLL reads exactly this shape.
    let text = serde_json::to_string(marker).unwrap();
    for field in [
        "\"telemetry\"",
        "\"token\"",
        "\"upload_endpoint\"",
        "\"session_id\"",
    ] {
        assert!(text.contains(field), "{field} missing from the marker");
    }
}

#[tokio::test]
async fn an_upload_address_the_login_servers_do_not_vouch_for_is_refused() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(reply("http://elsewhere.example/api/telemetry")),
        )
        .mount(&server)
        .await;
    assert_eq!(
        start(&identity(), &servers(&server.uri()), &[]).await,
        Err(Outcome::EndpointRefused)
    );
    // No login server at all means nowhere to ask.
    assert_eq!(
        start(&identity(), &[], &[]).await,
        Err(Outcome::SessionUnavailable)
    );
}

#[tokio::test]
async fn a_server_that_gives_no_usable_session_never_becomes_an_error() {
    for response in [
        ResponseTemplate::new(503),
        ResponseTemplate::new(200).set_body_string("not json"),
        ResponseTemplate::new(200).set_body_string("x".repeat(MAX_REPLY_FOR_TEST + 1)),
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "session_id": "s", "token": "", "expires_at_ms": 1, "upload_endpoint": "https://x.example/api",
            "chunk_max_bytes": 1, "flush_interval_ms": 1,
        })),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(response)
            .mount(&server)
            .await;
        assert_eq!(
            start(&identity(), &servers(&server.uri()), &[]).await,
            Err(Outcome::SessionUnavailable)
        );
    }
}
const MAX_REPLY_FOR_TEST: usize = 16 * 1024;

#[test]
fn only_short_switch_lists_reach_the_game_environment() {
    let passed = session::passthrough_for_test(|name| match name {
        "CIMMERIA_CLIENT_CAPTURE" => Some("unfilter,firehose".into()),
        "CIMMERIA_CLIENT_HOOKS_ENABLE" => Some("mercury_*".into()),
        "CIMMERIA_CLIENT_HOOKS_DISABLE" => Some("x; rm -rf".into()),
        _ => Some("never asked for".into()),
    });
    assert_eq!(
        passed,
        [
            ("CIMMERIA_CLIENT_CAPTURE", "unfilter,firehose".to_string()),
            ("CIMMERIA_CLIENT_HOOKS_ENABLE", "mercury_*".to_string()),
        ]
    );
    assert!(session::passthrough_for_test(|_| None).is_empty());
}

#[test]
fn the_last_launch_outcome_round_trips() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    assert_eq!(state.game_telemetry_status().unwrap(), None);
    let id = Uuid::from_u128(9);
    state
        .record_game_telemetry(id, Outcome::SessionUnavailable)
        .unwrap();
    assert_eq!(
        state.game_telemetry_status().unwrap(),
        Some(Status {
            schema_version: 1,
            operation_id: id,
            outcome: Outcome::SessionUnavailable
        })
    );
}
