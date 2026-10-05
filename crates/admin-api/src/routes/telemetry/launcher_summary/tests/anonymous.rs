//! The route is anonymous: the launcher's request carries no token and is
//! accepted, and an `Authorization` header a caller sends anyway is not
//! read. It changes no verdict, no refusal and no row, it is never written
//! to a log or a response, and a real token is worth exactly as much as
//! garbage.

use axum::http::header::AUTHORIZATION;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::Value;

use crate::routes::dev_session::{decode_token, load_secret};

use super::super::dto::{SummaryError, SummaryResponse, Verdict::Accepted};
use super::{
    batch, body_text, capture, content_type, element, json_headers, refusal, session_token,
    summary_rows, with_authorization, Env, Harness, Row, ENV_SECRET, REQUEST_ALL,
    REQUEST_INSTALL_FAILURE, REQUEST_MIXED,
};

/// Printable, and in no fixture: a row or a body holding it is an echo.
const NEEDLE: &str = "ZZMARKER";

/// The status and the body as the client would read them.
fn answer(result: Result<SummaryResponse, SummaryError>) -> (u16, String) {
    let response = match result {
        Ok(response) => Json(response).into_response(),
        Err(refusal) => refusal.into_response(),
    };
    (response.status().as_u16(), body_text(response))
}

/// The `Authorization` values tried: junk in several shapes, then a player
/// token and a lab token from the real mint. Each comes with the strings
/// that would give it away in a row or a body (the value itself and, for a
/// real token, the session and the install id it was minted for).
fn authorizations() -> Vec<(&'static str, String, Vec<String>)> {
    let secret = load_secret().unwrap();
    let mut cases: Vec<(&'static str, String, Vec<String>)> = [
        ("garbage bearer", format!("Bearer {NEEDLE}.garbage")),
        ("no scheme", NEEDLE.to_string()),
        ("basic", format!("Basic {NEEDLE}")),
        ("an empty bearer", "Bearer ".to_string()),
        ("an empty value", String::new()),
    ]
    .into_iter()
    .map(|(name, value)| (name, value, vec![NEEDLE.to_string()]))
    .collect();
    for (name, kind) in [("a player token", None), ("a lab token", Some("lab"))] {
        let token = session_token(kind);
        let claims = decode_token(&token, &secret).expect("a real, valid token");
        cases.push((
            name,
            format!("Bearer {token}"),
            vec![token, claims.sid, claims.sub],
        ));
    }
    cases
}

fn assert_absent(name: &str, needles: &[String], body: &str, rows: &[Row]) {
    for needle in needles.iter().filter(|needle| !needle.is_empty()) {
        assert!(!body.contains(needle.as_str()), "{name}: body {body:?}");
        for row in rows {
            assert!(!row.mentions(needle), "{name}: {row:#?}");
        }
    }
}

/// **The golden requests need no token.** `request-all.json` and the
/// recorded `request-install-failure.json` are accepted from a request
/// whose only header is `Content-Type`, on a server with no HMAC secret at
/// all: the route has no use for one.
#[test]
fn the_golden_requests_are_accepted_with_no_authorization_and_no_secret() {
    let _env = Env::install();
    std::env::remove_var(ENV_SECRET);
    assert!(
        load_secret().is_err(),
        "the server could not verify a token"
    );

    for (name, body, sent) in [
        ("request-all.json", REQUEST_ALL, 27),
        ("request-install-failure.json", REQUEST_INSTALL_FAILURE, 1),
    ] {
        let h = Harness::new();
        assert!(!h.headers.contains_key(AUTHORIZATION), "{name}");
        assert_eq!(h.headers.len(), 1, "{name}: only Content-Type");

        let (result, rows) = capture(|| h.post(body.as_bytes()));
        assert_eq!(result.expect(name).results, vec![Accepted; sent], "{name}");
        assert_eq!(summary_rows(&rows).len(), sent, "{name}");
    }
}

/// **An `Authorization` header changes nothing on an accepted request.**
/// `request-mixed.json` has one element of each verdict. Whatever the
/// header holds, the response and every row are exactly those of the same
/// request with no header (the control), and the header reaches neither.
#[test]
fn an_authorization_header_changes_no_verdict_and_reaches_no_row() {
    let _env = Env::install();
    let (control, control_rows) = capture(|| Harness::new().post(REQUEST_MIXED.as_bytes()));
    let control = answer(control);
    assert_eq!(control.0, 200);
    assert_eq!(
        serde_json::from_str::<Value>(&control.1).unwrap()["results"],
        serde_json::json!(["accepted", "duplicate", "rejected"])
    );
    assert_eq!(control_rows.len(), 5, "summary, three phases, batch");

    for (name, value, needles) in authorizations() {
        let mut h = Harness::new();
        h.headers = with_authorization(&value);
        let (result, rows) = capture(|| h.post(REQUEST_MIXED.as_bytes()));
        let got = answer(result);
        assert_eq!(got, control, "{name}");
        assert_eq!(rows, control_rows, "{name}");
        assert_absent(name, &needles, &got.1, &rows);
    }
}

/// **Nor on a refused one.** A request refused for its body (400) or its
/// content type (415) gets the same status and the same static body with
/// any `Authorization` header as with none, and still writes no row.
#[test]
fn an_authorization_header_changes_no_refusal() {
    let _env = Env::install();
    let valid = batch(vec![element(1)]).to_string().into_bytes();
    let faults: [(&str, Option<&str>, &[u8], u16); 2] = [
        ("a bad body", Some("application/json"), b"not json", 400),
        ("a wrong content type", Some("text/plain"), &valid, 415),
    ];
    for (fault, media_type, body, status) in faults {
        let mut h = Harness::new();
        h.headers = content_type(media_type);
        let control = answer(h.post(body));
        assert_eq!(control.0, status, "{fault}: control");

        for (name, value, needles) in authorizations() {
            let mut h = Harness::new();
            h.headers = content_type(media_type);
            h.headers
                .insert(AUTHORIZATION, value.parse().expect("a header value"));
            let (result, rows) = capture(|| h.post(body));
            let got = answer(result);
            assert_eq!(got, control, "{fault}, {name}");
            assert!(rows.is_empty(), "{fault}, {name}: {rows:#?}");
            assert_absent(name, &needles, &got.1, &rows);
        }
    }
}

/// **A token buys nothing.** With an allowance of one, the second request
/// is a 429 whether it carries a real player token or not, and with the
/// kill switch on a request carrying one is a 503. The first request, the
/// same one inside the allowance, is the control.
#[test]
fn a_real_token_is_no_way_past_the_quota_or_the_kill_switch() {
    let env = Env::install();
    let player = with_authorization(&format!("Bearer {}", session_token(None)));
    let post = |h: &Harness, n: u32| h.post_json(&batch(vec![element(n)]));

    for headers in [json_headers(), player.clone()] {
        let mut h = Harness::new();
        h.policy.per_ip = 1;
        h.headers = headers;
        assert_eq!(post(&h, 1).expect("control").results, [Accepted]);
        let r = refusal(post(&h, 2).unwrap_err());
        assert_eq!(r.status, 429, "{r:?}");
        assert_eq!(r.retry_after.as_deref(), Some("3601"));
    }

    let mut h = Harness::new();
    h.headers = player;
    env.set_kill_switch(true);
    let r = refusal(post(&h, 1).unwrap_err());
    assert_eq!(r.status, 503, "{r:?}");
    assert_eq!(r.retry_after.as_deref(), Some("60"));
    env.set_kill_switch(false);
    assert_eq!(post(&h, 1).expect("control").results, [Accepted]);
}
