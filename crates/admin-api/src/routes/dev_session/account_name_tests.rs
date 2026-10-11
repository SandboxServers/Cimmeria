//! The stream join (TG-NET-18, D-TG14): the mint signs the launcher's hashed
//! `machine_id` in as `mid` (and an `account_name`, when sent, as `acct`)
//! and records the stream with the request's address, so world entry can
//! say whether the session from that address has client telemetry.

use std::time::Instant;

use cimmeria_services::auth::client_telemetry::{client_telemetry_for, ClientTelemetry};

use super::handlers::{mint_inner, refresh_inner, DevSessionRequest, Tables};
use super::tests::{ip, policy, request, secret, status, EnvGuard, NOW_UNIX};
use super::token::{decode_token, AuthError};

// The stream registry is process-wide: each test uses its own machine ids,
// account names and addresses (192.0.2.0/24, which no other test mints from).

fn request_for(machine_id: &str, account_name: Option<&str>) -> DevSessionRequest {
    DevSessionRequest {
        machine_id: machine_id.to_string(),
        account_name: account_name.map(str::to_string),
        ..request("3f2504e0-4f89-41d3-9a0c-0305e82c3301")
    }
}

/// The join: a mint records a stream under its machine and address whose
/// `session_id` is the token's, and a refresh keeps the machine in the
/// signed claims and the stream alive. Reverting the mint's `record_stream`
/// call leaves the address `absent`.
#[test]
fn a_mint_records_the_stream_world_entry_joins_on_by_machine_and_address() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let at = ip("192.0.2.10");
    let resp = mint_inner(
        &t,
        &policy(10, 10, 10),
        at,
        request_for("tg18mint00000001", None),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let claims = decode_token(&resp.token, &secret()).unwrap();
    assert_eq!(claims.mid.as_deref(), Some("tg18mint00000001"));
    assert_eq!(claims.acct, None);
    assert_eq!(
        client_telemetry_for(None, Some(at), NOW_UNIX),
        ClientTelemetry::Present {
            session_id: resp.session_id.clone(),
            machine_id: Some("tg18mint00000001".into()),
        }
    );

    // Past the first token's expiry the stream lives on only if the refresh
    // re-recorded it with the new `exp`.
    let refreshed = refresh_inner(
        &t,
        &policy(10, 10, 10),
        at,
        &resp.token,
        Instant::now(),
        NOW_UNIX + 60,
    )
    .unwrap();
    let claims = decode_token(&refreshed.token, &secret()).unwrap();
    assert_eq!(claims.mid.as_deref(), Some("tg18mint00000001"));
    assert_eq!(
        client_telemetry_for(None, Some(at), claims.exp - 1).session_id(),
        Some(resp.session_id.as_str()),
        "a refresh keeps the stream alive to the new expiry"
    );
}

/// Mint A, mint B (a relaunch on the same machine), then A's delayed
/// refresh lands: B must stay the machine's stream.
#[test]
fn a_stale_refresh_does_not_overwrite_a_newer_mint() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let at = ip("192.0.2.11");
    let mint = |now| {
        mint_inner(
            &t,
            &policy(10, 10, 10),
            at,
            request_for("tg18stale0000001", None),
            Instant::now(),
            now,
        )
        .unwrap()
    };
    let a = mint(NOW_UNIX);
    let b = mint(NOW_UNIX + 100);
    refresh_inner(
        &t,
        &policy(10, 10, 10),
        at,
        &a.token,
        Instant::now(),
        NOW_UNIX + 50,
    )
    .unwrap();
    assert_eq!(
        client_telemetry_for(None, Some(at), NOW_UNIX + 60).session_id(),
        Some(b.session_id.as_str())
    );
}

/// A name, when a launcher sends one, is signed in as `acct` and wins the
/// join even from another address.
#[test]
fn a_mint_with_an_account_is_joined_by_account_first() {
    let _g = EnvGuard::install();
    let resp = mint_inner(
        &Tables::new(),
        &policy(10, 10, 10),
        ip("192.0.2.12"),
        request_for("tg18acct00000001", Some("tg18-mint-acct")),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let claims = decode_token(&resp.token, &secret()).unwrap();
    assert_eq!(claims.acct.as_deref(), Some("tg18-mint-acct"));
    assert_eq!(
        client_telemetry_for(Some("TG18-Mint-Acct"), Some(ip("192.0.2.200")), NOW_UNIX)
            .session_id(),
        Some(resp.session_id.as_str())
    );
}

/// An older launcher sends no `account_name` (or a blank one): it still
/// mints and its token has no `acct` key.
#[test]
fn a_mint_without_an_account_still_works() {
    let _g = EnvGuard::install();
    for name in [None, Some(""), Some("   ")] {
        let resp = mint_inner(
            &Tables::new(),
            &policy(10, 10, 10),
            ip("192.0.2.13"),
            request_for("tg18noacct000001", name),
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

/// A blank `machine_id` mints a token with no `mid` key.
#[test]
fn a_blank_machine_id_signs_no_mid() {
    let _g = EnvGuard::install();
    let resp = mint_inner(
        &Tables::new(),
        &policy(10, 10, 10),
        ip("192.0.2.14"),
        request_for("  ", None),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let claims = decode_token(&resp.token, &secret()).unwrap();
    assert_eq!(claims.mid, None);
}

/// A name with a line break would forge a line in the plain log sinks, and
/// one longer than the game login's 20-byte cap is a name the game would
/// refuse anyway. Exactly 20 bytes is accepted.
#[test]
fn a_malformed_account_name_is_refused() {
    let _g = EnvGuard::install();
    for bad in ["acct\nlevel=error msg=forged".to_string(), "a".repeat(21)] {
        let err = mint_inner(
            &Tables::new(),
            &policy(10, 10, 10),
            ip("192.0.2.15"),
            request_for("tg18bad000000001", Some(&bad)),
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
    assert_ne!(
        client_telemetry_for(Some("acct\nlevel=error msg=forged"), None, NOW_UNIX).as_str(),
        "present"
    );
    mint_inner(
        &Tables::new(),
        &policy(10, 10, 10),
        ip("192.0.2.15"),
        request_for("tg18bad000000001", Some(&"a".repeat(20))),
        Instant::now(),
        NOW_UNIX,
    )
    .expect("20 bytes is the game's own cap and is accepted");
}
