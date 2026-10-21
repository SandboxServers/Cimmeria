//! The launcher token at the gate: its kind and scope must agree, it may
//! post chunks but not bundles, it has its own session, install and address
//! quotas and its own slot pool, and the launcher kill switch refuses it
//! alone. The player's limits are the same as before.

use std::net::{IpAddr, Ipv4Addr};
use std::time::Instant;

use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;

use crate::routes::dev_session::{encode_token, AuthError};
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::upload_gate::{
    admit, Route, UploadLimits, UploadPolicy, UploadState, Uploader,
};

use super::{
    launcher_chunk_request, launcher_claims, launcher_token, small_chunk, token, Env, PEER,
};

/// A second address, for the tests that need one not already in flight.
const OTHER_PEER: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 8));

/// The headers a client sends: its bearer token.
fn bearer(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    headers
}

/// One admit from `peer` on `route`. An admitted request drops at once, so
/// its slot frees; only the refusal, if any, is kept.
fn outcome(
    state: &UploadState,
    policy: &UploadPolicy,
    route: Route,
    headers: &HeaderMap,
    peer: IpAddr,
) -> Result<(), IngestError> {
    admit(
        state,
        policy,
        route,
        headers,
        &mut Uploader::anonymous(peer),
        Instant::now(),
    )
    .map(drop)
}

/// **A launcher token cannot upload a bundle.** The bundle route answers 403
/// and charges nothing: with one bundle allowed per session and per address,
/// a player bundle from the same address still passes afterwards.
#[test]
fn a_launcher_token_cannot_upload_a_bundle() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy {
        bundle_per_ip: 1,
        bundle_per_session: 1,
        ..UploadPolicy::defaults()
    };
    let launcher = bearer(&launcher_token(
        "sess-launcher-bundle",
        "install-launcher-bundle",
    ));
    let err = outcome(&state, &policy, Route::Bundle, &launcher, PEER).unwrap_err();
    assert!(matches!(err, IngestError::Forbidden(_)), "{err:?}");
    assert_eq!(err.into_response().status(), StatusCode::FORBIDDEN);

    let player = bearer(&token("sess-player-bundle"));
    assert!(outcome(&state, &policy, Route::Bundle, &player, PEER).is_ok());
}

/// **The kind and the scope must agree.** A launcher kind with the player's
/// scope is refused on both routes, and a player kind with the launcher's
/// scope is refused on the bundle route.
#[test]
fn kind_and_scope_must_agree_at_the_gate() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy::defaults();

    let mut mismatched = launcher_claims("sess-mismatch", "install-mismatch");
    mismatched.scope = vec!["telemetry.write".into()];
    let headers = bearer(&encode_token(&mismatched, &[0x5c; 64]).unwrap());
    for route in [Route::Chunk, Route::Bundle] {
        let err = outcome(&state, &policy, route, &headers, PEER).unwrap_err();
        assert!(
            matches!(err, IngestError::Auth(AuthError::MissingScope { .. })),
            "{route:?}: {err:?}"
        );
    }

    let mut player = super::claims("sess-player-launcher-scope");
    player.scope = vec!["telemetry.launcher.write".into()];
    let headers = bearer(&encode_token(&player, &[0x5c; 64]).unwrap());
    let err = outcome(&state, &policy, Route::Bundle, &headers, PEER).unwrap_err();
    assert!(
        matches!(err, IngestError::Auth(AuthError::MissingScope { .. })),
        "{err:?}"
    );
}

/// **A launcher token may upload a chunk.**
#[test]
fn a_launcher_token_may_upload_a_chunk() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy::defaults();
    let request = launcher_chunk_request("sess-launcher-chunk", small_chunk(1));
    assert!(outcome(&state, &policy, Route::Chunk, request.headers(), PEER).is_ok());
}

/// **Launcher chunks have their own session quota.** The third chunk of a
/// session is refused; a player token from the same address still passes.
#[test]
fn launcher_chunks_have_their_own_session_quota() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy {
        launcher_chunk_per_session: 2,
        ..UploadPolicy::defaults()
    };
    let launcher = bearer(&launcher_token(
        "sess-launcher-quota",
        "install-launcher-quota",
    ));
    assert!(outcome(&state, &policy, Route::Chunk, &launcher, PEER).is_ok());
    assert!(outcome(&state, &policy, Route::Chunk, &launcher, PEER).is_ok());
    let err = outcome(&state, &policy, Route::Chunk, &launcher, PEER).unwrap_err();
    assert!(
        matches!(err, IngestError::Auth(AuthError::QuotaExceeded(_))),
        "{err:?}"
    );

    let player = bearer(&token("sess-player"));
    assert!(outcome(&state, &policy, Route::Chunk, &player, PEER).is_ok());
}

/// **Launcher chunks are capped per install.** Four sessions of one install
/// send one chunk each: the fourth is refused.
#[test]
fn launcher_chunks_are_capped_per_install() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy {
        launcher_chunk_per_install: 3,
        ..UploadPolicy::defaults()
    };
    for n in 0..3 {
        let session = format!("sess-install-{n}");
        let headers = bearer(&launcher_token(&session, "install-shared"));
        assert!(outcome(&state, &policy, Route::Chunk, &headers, PEER).is_ok());
    }
    let headers = bearer(&launcher_token("sess-install-3", "install-shared"));
    let err = outcome(&state, &policy, Route::Chunk, &headers, PEER).unwrap_err();
    assert!(
        matches!(err, IngestError::Auth(AuthError::QuotaExceeded(_))),
        "{err:?}"
    );
}

/// **Launcher chunks are capped per address.** Three sessions of three
/// installs from one address: the third is refused.
#[test]
fn launcher_chunks_are_capped_per_address() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy {
        launcher_chunk_per_ip: 2,
        ..UploadPolicy::defaults()
    };
    for n in 0..2 {
        let headers = bearer(&launcher_token(
            &format!("sess-address-{n}"),
            &format!("install-address-{n}"),
        ));
        assert!(outcome(&state, &policy, Route::Chunk, &headers, PEER).is_ok());
    }
    let headers = bearer(&launcher_token("sess-address-2", "install-address-2"));
    let err = outcome(&state, &policy, Route::Chunk, &headers, PEER).unwrap_err();
    assert!(
        matches!(err, IngestError::Auth(AuthError::QuotaExceeded(_))),
        "{err:?}"
    );
}

/// **Launcher chunks have their own slots.** With the launcher's one slot
/// held, a second launcher chunk is refused by the route's pool, and a
/// player chunk still passes, because the player's pool is not touched. The
/// second request comes from another address: the held launcher already has
/// this address's share of the pool.
#[test]
fn launcher_chunks_use_their_own_slots() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits {
        launcher_chunk_slots: 1,
        ..UploadLimits::default()
    });
    let policy = UploadPolicy::defaults();
    let held = admit(
        &state,
        &policy,
        Route::Chunk,
        &bearer(&launcher_token("sess-slot-held", "install-slot-held")),
        &mut Uploader::anonymous(PEER),
        Instant::now(),
    )
    .unwrap();

    let second = bearer(&launcher_token("sess-slot-second", "install-slot-second"));
    let err = outcome(&state, &policy, Route::Chunk, &second, OTHER_PEER).unwrap_err();
    assert!(matches!(err, IngestError::Busy { .. }), "{err:?}");
    assert_eq!(err_pool(&err), "route");

    let player = bearer(&token("sess-slot-player"));
    assert!(outcome(&state, &policy, Route::Chunk, &player, OTHER_PEER).is_ok());
    drop(held);
}

/// The pool a `Busy` refusal names.
fn err_pool(err: &IngestError) -> &'static str {
    match err {
        IngestError::Busy { pool, .. } => pool,
        other => panic!("not a busy refusal: {other:?}"),
    }
}

/// **Each kill switch refuses a launcher chunk.** The global switch refuses
/// everyone. The launcher switch refuses a launcher chunk and leaves a
/// player chunk alone.
#[test]
fn the_kill_switches_refuse_a_launcher_chunk() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let launcher = bearer(&launcher_token(
        "sess-kill-launcher",
        "install-kill-launcher",
    ));

    let global = UploadPolicy {
        kill_switch: true,
        ..UploadPolicy::defaults()
    };
    let err = outcome(&state, &global, Route::Chunk, &launcher, PEER).unwrap_err();
    assert!(
        matches!(err, IngestError::Auth(AuthError::KillSwitchActive)),
        "{err:?}"
    );

    let launcher_off = UploadPolicy {
        launcher_disabled: true,
        ..UploadPolicy::defaults()
    };
    let err = outcome(&state, &launcher_off, Route::Chunk, &launcher, PEER).unwrap_err();
    assert!(
        matches!(err, IngestError::Auth(AuthError::KillSwitchActive)),
        "{err:?}"
    );
    let player = bearer(&token("sess-kill-player"));
    assert!(outcome(&state, &launcher_off, Route::Chunk, &player, PEER).is_ok());
}

/// **The launcher's size caps.** `launcher()` takes the smaller of each
/// limit and the launcher's cap.
#[test]
fn launcher_limits_are_capped() {
    let limits = UploadLimits::default().launcher();
    assert_eq!(limits.chunk_body_bytes, 65_536);
    assert_eq!(limits.chunk_decompressed_bytes, 262_144);
    assert_eq!(limits.chunk_row_bytes, 4_096);
    assert_eq!(limits.chunk_rows, 200);
}

/// **The player's limits are unchanged.** With every launcher limit at 1, a
/// player's three chunks from one session all pass.
#[test]
fn player_limits_are_unchanged() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits {
        launcher_chunk_slots: 1,
        ..UploadLimits::default()
    });
    let policy = UploadPolicy {
        launcher_chunk_per_session: 1,
        launcher_chunk_per_install: 1,
        launcher_chunk_per_ip: 1,
        ..UploadPolicy::defaults()
    };
    let player = bearer(&token("sess-player-unchanged"));
    for _ in 0..3 {
        assert!(outcome(&state, &policy, Route::Chunk, &player, PEER).is_ok());
    }
}
