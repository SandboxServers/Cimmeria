//! Scope separation, through the real mint: a summary token works only on
//! the summary route, and a player or lab token only on the uploads.

use axum::http::HeaderMap;

use crate::routes::dev_session::{
    decode_token, encode_token, load_secret, AuthError, TokenClaims, SCOPE_LAUNCHER_SUMMARY_WRITE,
    SCOPE_TELEMETRY_WRITE,
};

use super::super::super::dto::IngestError;
use super::super::super::handlers::{verify_bearer, verify_bearer_scoped};
use super::super::dto::Verdict;
use super::{
    batch, bearer, capture, element, refusal, session_token, summary_rows, summary_token, Env,
    Harness,
};

fn missing_scope(result: Result<TokenClaims, IngestError>) -> &'static str {
    match result {
        Err(IngestError::Auth(AuthError::MissingScope { wanted })) => wanted,
        Ok(claims) => panic!("a token scoped {:?} was accepted", claims.scope),
        Err(other) => panic!("expected MissingScope, got {other:?}"),
    }
}

/// The upload routes' check refuses a summary token for want of
/// `telemetry.write`, and accepts a player's (the control).
#[test]
fn a_summary_token_is_refused_by_the_upload_routes() {
    let _env = Env::install();
    assert_eq!(
        missing_scope(verify_bearer(&bearer(&summary_token()))),
        "telemetry.write"
    );
    let player = verify_bearer(&bearer(&session_token(None))).expect("control: a player token");
    assert!(player.has_scope(SCOPE_TELEMETRY_WRITE));
}

/// The summary route's check refuses a player token and a lab token for
/// want of `launcher_summary.write`, and accepts a summary token (the
/// control).
#[test]
fn player_and_lab_tokens_are_refused_by_the_summary_scope_check() {
    let _env = Env::install();
    for kind in [None, Some("lab")] {
        let token = session_token(kind);
        assert_eq!(
            missing_scope(verify_bearer_scoped(
                &bearer(&token),
                SCOPE_LAUNCHER_SUMMARY_WRITE
            )),
            "launcher_summary.write",
            "{kind:?}"
        );
    }
    let summary = verify_bearer_scoped(&bearer(&summary_token()), SCOPE_LAUNCHER_SUMMARY_WRITE)
        .expect("control: a summary token");
    assert_eq!(summary.scope, ["launcher_summary.write"]);
}

/// The same separation at the handler: a valid body under a player or lab
/// token is a 401 that writes no row and remembers no id, so the same body
/// under a summary token is then accepted as new.
///
/// Dropping the scope check from the bearer verification lets the player
/// token in and fails the first assertion.
#[test]
fn the_summary_route_refuses_a_player_token_and_accepts_a_summary_one() {
    let _env = Env::install();
    let mut h = Harness::new();
    let summary = h.headers.clone();
    let body = batch(vec![element(1)]);

    for kind in [None, Some("lab")] {
        h.headers = bearer(&session_token(kind));
        let (result, rows) = capture(|| h.post_json(&body));
        let r = refusal(result.expect_err("a telemetry.write token on the summary route"));
        assert_eq!(r.status, 401, "{kind:?}: {r:?}");
        assert_eq!(r.body, "Token is not scoped for launcher_summary.write");
        assert!(rows.is_empty(), "{kind:?}: {rows:#?}");
    }

    h.headers = summary;
    let (results, rows) = capture(|| h.verdicts(&body));
    assert_eq!(results, [Verdict::Accepted]);
    assert_eq!(summary_rows(&rows).len(), 1);
}

/// What else the token check refuses, each as a 401 with a static body: no
/// header, a scheme other than `Bearer`, an empty token, a token that is
/// not one, a token signed with another key, and an expired summary token.
/// The unexpired twin of the last one is the control.
#[test]
fn a_missing_malformed_forged_or_expired_token_is_a_401() {
    let _env = Env::install();
    let mut h = Harness::new();
    let body = batch(vec![element(1)]);
    let secret = load_secret().unwrap();
    let now = chrono::Utc::now().timestamp();
    let minted = decode_token(&summary_token(), &secret).unwrap();
    let with_exp = |exp: i64, key: &[u8]| {
        let claims = TokenClaims {
            exp,
            ..minted.clone()
        };
        bearer(&encode_token(&claims, key).unwrap())
    };
    let mut basic = HeaderMap::new();
    basic.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Basic dXNlcjpwYXNz"),
    );

    let missing = "Missing or malformed Authorization header";
    let cases: [(&str, HeaderMap, &str); 6] = [
        ("no header", HeaderMap::new(), missing),
        ("another scheme", basic, missing),
        ("empty token", bearer(""), missing),
        (
            "not a token",
            bearer("not-a-token"),
            "Token payload decode failed",
        ),
        (
            "another key",
            with_exp(now + 600, &[0x11; 64]),
            "Token signature invalid",
        ),
        ("expired", with_exp(now - 1, &secret), ""),
    ];
    for (name, headers, text) in cases {
        h.headers = headers;
        let r = refusal(h.post_json(&body).expect_err(name));
        assert_eq!(r.status, 401, "{name}: {r:?}");
        if text.is_empty() {
            assert!(r.body.starts_with("Token expired"), "{name}: {r:?}");
        } else {
            assert_eq!(r.body, text, "{name}");
        }
    }

    h.headers = with_exp(now + 600, &secret);
    assert_eq!(h.verdicts(&body), [Verdict::Accepted], "control");
}
