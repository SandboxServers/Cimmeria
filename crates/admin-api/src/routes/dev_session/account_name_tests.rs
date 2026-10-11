//! The `account_name` request field (TG-NET-18, D-TG14): the launcher names
//! the game account, the mint signs it into the token as `acct` and records
//! the stream, so world entry can say whether the account has client
//! telemetry. A launcher that sends no name (every launcher before it) still
//! mints a token with its old bytes, and records nothing.

use std::time::Instant;

use cimmeria_services::auth::client_telemetry::{client_telemetry_for, ClientTelemetry};

use super::handlers::{mint_inner, refresh_inner, DevSessionRequest, Tables};
use super::tests::{ip, policy, request, secret, status, EnvGuard, NOW_UNIX};
use super::token::{decode_token, AuthError};

// The stream registry is process-wide: each test uses its own account name.

fn request_for(account_name: Option<&str>) -> DevSessionRequest {
    DevSessionRequest {
        account_name: account_name.map(str::to_string),
        ..request("3f2504e0-4f89-41d3-9a0c-0305e82c3301")
    }
}

/// The join: a mint that names the account records a stream under it whose
/// `session_id` is the token's, and a refresh keeps the account in the
/// signed claims and the stream alive. Reverting the mint's `record_stream`
/// call leaves the account `absent`.
#[test]
fn a_mint_with_an_account_records_the_stream_world_entry_joins_on() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let resp = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.7"),
        request_for(Some("tg18-mint-acct")),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let claims = decode_token(&resp.token, &secret()).unwrap();
    assert_eq!(claims.acct.as_deref(), Some("tg18-mint-acct"));
    assert_eq!(
        client_telemetry_for(Some("tg18-mint-acct"), NOW_UNIX),
        ClientTelemetry::Present {
            session_id: resp.session_id.clone()
        }
    );

    // Past the first token's expiry the stream lives on only if the refresh
    // re-recorded it with the new `exp`.
    let refreshed = refresh_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.7"),
        &resp.token,
        Instant::now(),
        NOW_UNIX + 60,
    )
    .unwrap();
    let claims = decode_token(&refreshed.token, &secret()).unwrap();
    assert_eq!(claims.acct.as_deref(), Some("tg18-mint-acct"));
    assert_eq!(
        client_telemetry_for(Some("tg18-mint-acct"), claims.exp - 1).session_id(),
        Some(resp.session_id.as_str()),
        "a refresh keeps the stream alive to the new expiry"
    );
}

/// An older launcher sends no `account_name` (or a blank one): it still
/// mints, its token has no `acct` key, and no stream is recorded.
#[test]
fn a_mint_without_an_account_still_works_and_records_nothing() {
    let _g = EnvGuard::install();
    for name in [None, Some(""), Some("   ")] {
        let resp = mint_inner(
            &Tables::new(),
            &policy(10, 10, 10),
            ip("203.0.113.8"),
            request_for(name),
            Instant::now(),
            NOW_UNIX,
        )
        .unwrap_or_else(|e| panic!("{name:?} must mint: {e}"));
        let claims = decode_token(&resp.token, &secret()).unwrap();
        assert_eq!(claims.acct, None, "requested {name:?}");
    }
    let old_body: DevSessionRequest = serde_json::from_str(
        r#"{"install_id":"i","machine_id":"m","branch":"b","git_sha":"g","launcher_version":"v"}"#,
    )
    .unwrap();
    assert_eq!(old_body.account_name, None);
}

/// A name with a line break would forge a line in the plain log sinks, and
/// an oversized one is storage an unauthenticated caller should not get.
#[test]
fn a_malformed_account_name_is_refused() {
    let _g = EnvGuard::install();
    for bad in ["acct\nlevel=error msg=forged".to_string(), "a".repeat(65)] {
        let err = mint_inner(
            &Tables::new(),
            &policy(10, 10, 10),
            ip("203.0.113.9"),
            request_for(Some(&bad)),
            Instant::now(),
            NOW_UNIX,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                AuthError::BadField {
                    field: "account_name",
                    ..
                }
            ),
            "{err:?}"
        );
        assert_eq!(status(err), axum::http::StatusCode::BAD_REQUEST);
    }
    assert_eq!(
        client_telemetry_for(Some("acct\nlevel=error msg=forged"), NOW_UNIX),
        ClientTelemetry::Absent
    );
}
