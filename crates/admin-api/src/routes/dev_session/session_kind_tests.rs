//! The `session_kind` request field and the `kind` claim it becomes: a lab
//! session is minted, refreshed and uploaded as lab, a player session
//! stays byte-identical to one minted before the claim existed, and an
//! unknown kind is refused rather than filed as a player.

use std::time::Instant;

use super::handlers::{mint_inner, refresh_inner, DevSessionRequest, Tables};
use super::tests::{ip, policy, request, secret, status, EnvGuard, NOW_UNIX};
use super::token::{decode_token, AuthError};

fn request_of_kind(kind: Option<&str>) -> DevSessionRequest {
    DevSessionRequest {
        session_kind: kind.map(str::to_string),
        ..request("cimmeria-lab")
    }
}

/// The lab supervisor asks for `session_kind = "lab"` and gets a token
/// whose signed `kind` claim says so; a refresh keeps it.
#[test]
fn a_lab_mint_carries_the_lab_kind_through_refresh() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let resp = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("127.0.0.1"),
        request_of_kind(Some("lab")),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let claims = decode_token(&resp.token, &secret()).unwrap();
    assert_eq!(claims.kind.as_deref(), Some("lab"));
    assert!(claims.is_lab());

    let refreshed = refresh_inner(
        &t,
        &policy(10, 10, 10),
        ip("127.0.0.1"),
        &resp.token,
        Instant::now(),
        NOW_UNIX + 60,
    )
    .unwrap();
    let claims = decode_token(&refreshed.token, &secret()).unwrap();
    assert!(claims.is_lab(), "a refresh must not demote a lab session");
}

/// A launcher that sends no kind, or `"player"`, gets a token with no
/// `kind` key: the payload is what it was before the claim existed.
#[test]
fn a_player_mint_has_no_kind_claim() {
    let _g = EnvGuard::install();
    for kind in [None, Some("player")] {
        let t = Tables::new();
        let resp = mint_inner(
            &t,
            &policy(10, 10, 10),
            ip("203.0.113.5"),
            request_of_kind(kind),
            Instant::now(),
            NOW_UNIX,
        )
        .unwrap();
        let claims = decode_token(&resp.token, &secret()).unwrap();
        assert_eq!(claims.kind, None, "requested {kind:?}");
        assert_eq!(claims.session_kind(), "player");
    }
}

/// An unknown kind is a 400, never a silent player session: a typo in the
/// lab config would otherwise file every lab row as a player's.
#[test]
fn an_unknown_session_kind_is_refused() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let err = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.5"),
        request_of_kind(Some("Lab")),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            AuthError::BadField {
                field: "session_kind",
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(status(err), axum::http::StatusCode::BAD_REQUEST);
}

/// The colo compose passes `${CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT:-}`, so an
/// operator who has not set it yet gives the server an empty string. The
/// mint must still hand back a usable endpoint, not `""`.
#[test]
fn a_blank_upload_endpoint_falls_back_to_the_default() {
    use super::handlers::upload_endpoint_from;
    let default = upload_endpoint_from(None);
    assert_eq!(default, "http://localhost:8443/api/telemetry");
    assert_eq!(upload_endpoint_from(Some(String::new())), default);
    assert_eq!(upload_endpoint_from(Some("  ".into())), default);
    assert_eq!(
        upload_endpoint_from(Some("https://t.example/api/telemetry ".into())),
        "https://t.example/api/telemetry"
    );
}

/// The request body a lab supervisor sends deserializes with the kind,
/// and a launcher's body without the field still deserializes.
#[test]
fn the_request_body_accepts_and_defaults_the_kind() {
    let lab: DevSessionRequest = serde_json::from_str(
        r#"{"install_id":"cimmeria-lab","machine_id":"m","branch":"lab","git_sha":"g",
            "launcher_version":"lab","tags":["lab"],"session_kind":"lab"}"#,
    )
    .unwrap();
    assert_eq!(lab.session_kind.as_deref(), Some("lab"));
    let launcher: DevSessionRequest = serde_json::from_str(
        r#"{"install_id":"i","machine_id":"m","branch":"b","git_sha":"g","launcher_version":"v"}"#,
    )
    .unwrap();
    assert_eq!(launcher.session_kind, None);
}
