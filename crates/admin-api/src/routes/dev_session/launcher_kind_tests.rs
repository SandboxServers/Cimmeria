//! The `launcher` session kind (C12, AU-11): its 15-minute lifetime and its
//! own scope, its refusals, its own quotas and kill switch, and the player
//! and lab mints that must stay as they were.

use std::time::Instant;

use cimmeria_services::auth::client_telemetry::{client_telemetry_for, ClientTelemetry};

use super::handlers::{mint_inner, refresh_inner, DevSessionRequest, QuotaPolicy, Tables};
use super::tests::{ip, policy, request, secret, status, EnvGuard, NOW_UNIX};
use super::token::{decode_token, AuthError, TokenClaims, SCOPE_LAUNCHER_WRITE};
use super::SCOPE_TELEMETRY_WRITE;

fn launcher_request(install_id: &str) -> DevSessionRequest {
    DevSessionRequest {
        session_kind: Some("launcher".into()),
        ..request(install_id)
    }
}

fn lab_request(install_id: &str) -> DevSessionRequest {
    DevSessionRequest {
        session_kind: Some("lab".into()),
        ..request(install_id)
    }
}

/// A launcher mint lives 15 minutes, carries only the launcher scope and
/// signs neither an account nor a machine id, and gets the small chunk size.
#[test]
fn a_launcher_mint_is_short_lived_scoped_and_carries_no_account_or_machine() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let req = DevSessionRequest {
        machine_id: "m-1".into(),
        ..launcher_request("cimmeria-launcher")
    };
    let resp = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.9"),
        req,
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let claims = decode_token(&resp.token, &secret()).unwrap();
    assert_eq!(claims.kind.as_deref(), Some("launcher"));
    assert!(claims.is_launcher());
    assert!(claims.kind_matches_scope());
    assert_eq!(claims.exp - claims.iat, 900);
    assert_eq!(claims.scope, [SCOPE_LAUNCHER_WRITE]);
    assert_eq!(claims.acct, None);
    assert_eq!(claims.mid, None);
    assert_eq!(resp.chunk_max_bytes, 65_536);
}

/// A launcher names no account: a non-blank `account_name` is a 400.
#[test]
fn a_launcher_mint_with_an_account_name_is_refused() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let req = DevSessionRequest {
        account_name: Some("gerger".into()),
        ..launcher_request("cimmeria-launcher")
    };
    let err = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.9"),
        req,
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

/// A launcher token is never refreshed. The refusal comes before the refresh
/// quota is charged: a player session on the same tables then refreshes as
/// many times as `refresh_per_ip` allows, with none refused.
#[test]
fn a_launcher_token_is_not_refreshed() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(10, 10, 2);
    let launcher = mint_inner(
        &t,
        &p,
        ip("203.0.113.9"),
        launcher_request("cimmeria-launcher"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let err = refresh_inner(
        &t,
        &p,
        ip("203.0.113.9"),
        &launcher.token,
        Instant::now(),
        NOW_UNIX + 60,
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

    let player = mint_inner(
        &t,
        &p,
        ip("203.0.113.9"),
        request("3f2504e0-4f89-41d3-9a0c-0305e82c3301"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let mut token = player.token;
    for step in [60, 120] {
        let refreshed = refresh_inner(
            &t,
            &p,
            ip("203.0.113.9"),
            &token,
            Instant::now(),
            NOW_UNIX + step,
        )
        .expect("the refused launcher refresh must not use up refresh_per_ip");
        token = refreshed.token;
    }
}

/// The launcher quota per address is its own table: the fourth launcher
/// mint from one address is refused, while a player mint still passes.
#[test]
fn launcher_mints_have_their_own_ip_quota() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = QuotaPolicy {
        mint_launcher_per_install: 100,
        mint_launcher_per_ip: 3,
        ..policy(100, 10, 10)
    };
    let at = ip("203.0.113.20");
    for n in 1..=3 {
        let req = launcher_request(&format!("3f2504e0-4f89-41d3-9a0c-0305e82c330{n}"));
        mint_inner(&t, &p, at, req, Instant::now(), NOW_UNIX)
            .unwrap_or_else(|e| panic!("launcher mint {n}: {e:?}"));
    }
    let err = mint_inner(
        &t,
        &p,
        at,
        launcher_request("3f2504e0-4f89-41d3-9a0c-0305e82c3304"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::QuotaExceeded(_)), "{err:?}");
    mint_inner(
        &t,
        &p,
        at,
        request("3f2504e0-4f89-41d3-9a0c-0305e82c3305"),
        Instant::now(),
        NOW_UNIX,
    )
    .expect("a player mint from the same address still passes");
}

/// A launcher mint its own limit refuses spends no shared mint allowance (S1):
/// after 30 admitted and 10 refused launcher mints, a player mint from the
/// same address still passes the shared 32-per-hour table.
#[test]
fn refused_launcher_mints_do_not_spend_the_shared_mint_allowance() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = QuotaPolicy {
        mint_launcher_per_install: 100,
        mint_launcher_per_ip: 30,
        ..policy(32, 10, 10)
    };
    let at = ip("203.0.113.40");
    for n in 1..=40u32 {
        let req = launcher_request(&format!("3f2504e0-4f89-41d3-9a0c-0305e82c{n:04}"));
        let admitted = mint_inner(&t, &p, at, req, Instant::now(), NOW_UNIX).is_ok();
        assert_eq!(admitted, n <= 30, "launcher mint {n}");
    }
    mint_inner(
        &t,
        &p,
        at,
        request("3f2504e0-4f89-41d3-9a0c-0305e82c9999"),
        Instant::now(),
        NOW_UNIX,
    )
    .expect("a player mint from the same address still passes");
}

/// A burst of player mints does not spend the launcher allowance: the launcher
/// table's per-address limit of three still admits three launcher mints.
#[test]
fn a_player_mint_burst_does_not_spend_the_launcher_allowance() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = QuotaPolicy {
        mint_launcher_per_install: 100,
        mint_launcher_per_ip: 3,
        ..policy(100, 100, 100)
    };
    let at = ip("203.0.113.41");
    for n in 1..=10u32 {
        let req = request(&format!("3f2504e0-4f89-41d3-9a0c-0305e82d{n:04}"));
        mint_inner(&t, &p, at, req, Instant::now(), NOW_UNIX).expect("player mint");
    }
    for n in 1..=3u32 {
        let req = launcher_request(&format!("3f2504e0-4f89-41d3-9a0c-0305e82c{n:04}"));
        mint_inner(&t, &p, at, req, Instant::now(), NOW_UNIX)
            .unwrap_or_else(|e| panic!("launcher mint {n}: {e:?}"));
    }
}

/// The launcher quota per install is its own table: the third launcher mint
/// for one install is refused, while a player mint for it still passes.
#[test]
fn launcher_mints_have_their_own_install_quota() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = QuotaPolicy {
        mint_launcher_per_install: 2,
        mint_launcher_per_ip: 100,
        ..policy(100, 10, 10)
    };
    let install = "3f2504e0-4f89-41d3-9a0c-0305e82c3301";
    for n in 1..=2 {
        mint_inner(
            &t,
            &p,
            ip("203.0.113.30"),
            launcher_request(install),
            Instant::now(),
            NOW_UNIX,
        )
        .unwrap_or_else(|e| panic!("launcher mint {n}: {e:?}"));
    }
    let err = mint_inner(
        &t,
        &p,
        ip("203.0.113.30"),
        launcher_request(install),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::QuotaExceeded(_)), "{err:?}");
    mint_inner(
        &t,
        &p,
        ip("203.0.113.30"),
        request(install),
        Instant::now(),
        NOW_UNIX,
    )
    .expect("a player mint for the same install still passes");
}

/// Both kill switches refuse a launcher mint: the master switch, and the
/// launcher-only one with the master switch off. A player mint passes the
/// launcher-only switch.
#[test]
fn the_kill_switches_refuse_a_launcher_mint() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let at = ip("203.0.113.40");

    std::env::set_var("CIMMERIA_TELEMETRY_KILL_SWITCH", "1");
    let err = mint_inner(
        &t,
        &policy(10, 10, 10),
        at,
        launcher_request("cimmeria-launcher"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap_err();
    std::env::remove_var("CIMMERIA_TELEMETRY_KILL_SWITCH");
    assert!(matches!(err, AuthError::KillSwitchActive), "{err:?}");

    let p = QuotaPolicy {
        launcher_disabled: true,
        ..policy(10, 10, 10)
    };
    let err = mint_inner(
        &t,
        &p,
        at,
        launcher_request("cimmeria-launcher"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::KillSwitchActive), "{err:?}");
    mint_inner(
        &t,
        &p,
        at,
        request("3f2504e0-4f89-41d3-9a0c-0305e82c3301"),
        Instant::now(),
        NOW_UNIX,
    )
    .expect("the launcher-only switch leaves a player mint alone");
}

/// Launcher metadata must be printable ASCII: a control character in the
/// version is a 400 on a launcher mint, and the same value on a player mint
/// is not refused by this rule.
#[test]
fn launcher_metadata_must_be_printable_ascii() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let bad = "1.0\u{202E}x";
    let launcher = DevSessionRequest {
        launcher_version: bad.into(),
        ..launcher_request("cimmeria-launcher")
    };
    let err = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.50"),
        launcher,
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            AuthError::BadField {
                field: "launcher_version",
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(status(err), axum::http::StatusCode::BAD_REQUEST);

    let player = DevSessionRequest {
        launcher_version: bad.into(),
        ..request("3f2504e0-4f89-41d3-9a0c-0305e82c3302")
    };
    if let Err(e) = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.50"),
        player,
        Instant::now(),
        NOW_UNIX,
    ) {
        assert!(
            !e.to_string().contains("printable ASCII"),
            "the launcher rule must not reach a player mint: {e:?}"
        );
    }
}

/// The lab and player mints keep their lifetimes, scope and chunk size.
#[test]
fn lab_and_player_mints_are_unchanged() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let lab = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.60"),
        lab_request("cimmeria-lab"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let claims = decode_token(&lab.token, &secret()).unwrap();
    assert_eq!(claims.exp - claims.iat, 8 * 3600);
    assert_eq!(claims.scope, [SCOPE_TELEMETRY_WRITE]);

    let player = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.61"),
        request("3f2504e0-4f89-41d3-9a0c-0305e82c3303"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    assert_eq!(player.chunk_max_bytes, 1_048_576);
}

fn claims_of(kind: Option<&str>, scope: &str) -> TokenClaims {
    TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "3f2504e0-4f89-41d3-9a0c-0305e82c3301".into(),
        sid: "s".into(),
        iat: NOW_UNIX,
        exp: NOW_UNIX + 900,
        scope: vec![scope.into()],
        kind: kind.map(str::to_string),
        acct: None,
        mid: None,
    }
}

/// A kind and its scope must agree: a launcher with the shared scope and a
/// player with the launcher scope both fail; the matching pairs pass.
#[test]
fn kind_and_scope_must_agree() {
    assert!(!claims_of(Some("launcher"), SCOPE_TELEMETRY_WRITE).kind_matches_scope());
    assert!(!claims_of(None, SCOPE_LAUNCHER_WRITE).kind_matches_scope());
    assert!(claims_of(Some("launcher"), SCOPE_LAUNCHER_WRITE).kind_matches_scope());
    assert!(claims_of(None, SCOPE_TELEMETRY_WRITE).kind_matches_scope());
}

/// A launcher mint records no stream: it has no game to join (R4). The
/// address is one no other test uses, since the registry is process-wide.
#[test]
fn a_launcher_mint_records_no_stream() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let at = ip("192.0.2.211");
    mint_inner(
        &t,
        &policy(10, 10, 10),
        at,
        launcher_request("cimmeria-launcher"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    assert_eq!(
        client_telemetry_for(None, Some(at), NOW_UNIX),
        ClientTelemetry::Absent
    );
}
