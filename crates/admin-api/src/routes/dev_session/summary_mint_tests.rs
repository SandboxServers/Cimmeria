//! The launcher-summary arm of the mint: what the token carries, what the
//! request may carry, what reaches the log, and which quota it spends.
//! Every test drives the real `mint_inner` with fresh tables.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use super::handlers::{mint_inner, refresh_inner, DevSessionRequest, Tables};
use super::summary_mint::parse_version_triple;
use super::tests::{ip, policy, request, secret, status, EnvGuard, NOW_UNIX};
use super::token::{
    decode_token, AuthError, SCOPE_LAUNCHER_SUMMARY_WRITE, SCOPE_TELEMETRY_WRITE,
    SESSION_KIND_LAUNCHER_SUMMARY,
};

/// The exact mint body the desktop launcher's exporter sends, shared with
/// its tests so neither side can drift alone.
const MINT_REQUEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../launcher/desktop/engine/src/storage/launcher_summary/fixtures/mint-request.json"
));
/// `install_id` of that body.
const FIXTURE_INSTALL_ID: &str = "00000000-0000-4000-8000-0000000000aa";

fn summary_request() -> DevSessionRequest {
    serde_json::from_str(MINT_REQUEST).expect("the golden mint body is a DevSessionRequest")
}

fn mint(tables: &Tables, req: DevSessionRequest) -> Result<String, AuthError> {
    mint_inner(
        tables,
        &policy(10, 10, 10),
        ip("203.0.113.5"),
        req,
        Instant::now(),
        NOW_UNIX,
    )
    .map(|resp| resp.token)
}

/// The decoded payload of a token, as the JSON text that is signed.
fn payload_json(token: &str) -> String {
    let (payload, _) = token.split_once('.').unwrap();
    String::from_utf8(URL_SAFE_NO_PAD.decode(payload).unwrap()).unwrap()
}

/// **The golden mint.** The launcher's exact body mints a token whose only
/// scope is the summary one, whose kind and subject are the server
/// constant, and whose signed payload does not hold the `install_id` it was
/// minted with. A player mint with the same `install_id` is the control: it
/// gets `telemetry.write` and carries the id as its subject.
///
/// Issuing `telemetry.write` from the summary arm fails the scope
/// assertion; using `req.install_id` as the subject fails the next two.
#[test]
fn the_golden_summary_mint_carries_only_the_summary_scope_and_no_caller_value() {
    let _g = EnvGuard::install();
    let req = summary_request();
    assert_eq!(req.install_id, FIXTURE_INSTALL_ID);
    assert_eq!(
        req.session_kind.as_deref(),
        Some(SESSION_KIND_LAUNCHER_SUMMARY)
    );

    let token = mint(&Tables::new(), req).unwrap();
    let claims = decode_token(&token, &secret()).unwrap();
    assert_eq!(claims.scope, vec![SCOPE_LAUNCHER_SUMMARY_WRITE.to_string()]);
    assert_eq!(claims.scope, ["launcher_summary.write"]);
    assert_eq!(claims.kind.as_deref(), Some("launcher_summary"));
    assert_eq!(claims.sub, "launcher_summary");
    assert!(!claims.has_scope(SCOPE_TELEMETRY_WRITE));
    assert!(
        !payload_json(&token).contains(FIXTURE_INSTALL_ID),
        "the summary token must not carry the install_id"
    );

    let player = DevSessionRequest {
        install_id: FIXTURE_INSTALL_ID.into(),
        ..request("unused")
    };
    let token = mint(&Tables::new(), player).unwrap();
    let claims = decode_token(&token, &secret()).unwrap();
    assert_eq!(claims.scope, vec![SCOPE_TELEMETRY_WRITE.to_string()]);
    assert_eq!(claims.kind, None);
    assert_eq!(claims.sub, FIXTURE_INSTALL_ID);
    assert!(payload_json(&token).contains(FIXTURE_INSTALL_ID));
}

/// A refresh keeps what the mint issued: the summary scope and nothing
/// else, the kind, the constant subject. Refresh is how a token's life is
/// extended, so it must not be a way to widen it.
#[test]
fn a_refreshed_summary_token_keeps_only_the_summary_scope() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let token = mint(&t, summary_request()).unwrap();
    let refreshed = refresh_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.5"),
        &token,
        Instant::now(),
        NOW_UNIX + 60,
    )
    .unwrap();
    let claims = decode_token(&refreshed.token, &secret()).unwrap();
    assert_eq!(claims.scope, ["launcher_summary.write"]);
    assert_eq!(claims.kind.as_deref(), Some("launcher_summary"));
    assert_eq!(claims.sub, "launcher_summary");
}

/// A summary session may send no identifier and no free-form version: each
/// non-empty identifier field, any tag, and each malformed version is a
/// 400 naming the field. The untouched golden body is the control.
#[test]
fn a_summary_mint_refuses_identifiers_and_a_malformed_version() {
    let _g = EnvGuard::install();
    mint(&Tables::new(), summary_request()).expect("control: the golden body mints");

    type Mutation = fn(&mut DevSessionRequest);
    let identifiers: [(&str, Mutation); 4] = [
        ("machine_id", |r| r.machine_id = "DESKTOP-1".into()),
        ("branch", |r| r.branch = "main".into()),
        ("git_sha", |r| r.git_sha = "0123456".into()),
        ("tags", |r| r.tags = vec!["beta".into()]),
    ];
    for (field, mutate) in identifiers {
        let mut req = summary_request();
        mutate(&mut req);
        let err = mint(&Tables::new(), req).unwrap_err();
        assert!(
            matches!(err, AuthError::BadField { field: f, .. } if f == field),
            "{field}: {err:?}"
        );
        assert_eq!(status(err), axum::http::StatusCode::BAD_REQUEST, "{field}");
    }

    for version in [
        "",
        "0.1",
        "0.1.0.0",
        "1000.0.0",
        "0.1.x",
        "0.1.0-beta",
        "+1.0.0",
        "-1.0.0",
        " 0.1.0",
        "0.1.0\n",
        "0..0",
        "\u{0660}.\u{0661}.\u{0660}",
        "\u{ff10}.1.0",
    ] {
        let req = DevSessionRequest {
            launcher_version: version.into(),
            ..summary_request()
        };
        let err = mint(&Tables::new(), req).unwrap_err();
        assert!(
            matches!(
                err,
                AuthError::BadField {
                    field: "launcher_version",
                    ..
                }
            ),
            "{version:?}: {err:?}"
        );
        assert_eq!(
            status(err),
            axum::http::StatusCode::BAD_REQUEST,
            "{version:?}"
        );
    }
}

/// The version rule on its own: three components of one to three ASCII
/// digits, returned as integers.
#[test]
fn parse_version_triple_reads_three_short_ascii_components() {
    assert_eq!(parse_version_triple("0.1.0"), Some((0, 1, 0)));
    assert_eq!(parse_version_triple("999.999.999"), Some((999, 999, 999)));
    assert_eq!(parse_version_triple("007.01.000"), Some((7, 1, 0)));
    for bad in [
        "", "1", "1.2", "1.2.3.4", "1.2.", ".1.2", "1000.0.0", "1.2.3 ",
    ] {
        assert_eq!(parse_version_triple(bad), None, "{bad:?}");
    }
}

#[derive(Debug, Clone)]
struct Row {
    level: tracing::Level,
    fields: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
struct Rows(Arc<Mutex<Vec<Row>>>);

struct FieldText<'a>(&'a mut BTreeMap<String, String>);

impl Visit for FieldText<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

impl<S: Subscriber> Layer<S> for Rows {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut fields = BTreeMap::new();
        event.record(&mut FieldText(&mut fields));
        self.0.lock().unwrap().push(Row {
            level: *event.metadata().level(),
            fields,
        });
    }
}

/// Every event the closure emits, at every level (the layer has no filter).
fn capture(f: impl FnOnce()) -> Vec<Row> {
    let rows = Rows::default();
    let sub = tracing_subscriber::registry().with(rows.clone());
    tracing::subscriber::with_default(sub, f);
    let out = rows.0.lock().unwrap().clone();
    out
}

/// A summary mint writes one INFO row holding the session id, the kind,
/// the version re-formatted from its parsed integers, and the expiry. The
/// `install_id` it was minted with appears nowhere, and there is no
/// identifiers row.
///
/// The control is a player mint with the same `install_id`: its DEBUG
/// identifiers row does carry it, so the capture would have seen it.
#[test]
fn a_summary_mint_logs_no_install_id_and_no_caller_string() {
    let _g = EnvGuard::install();
    const INSTALL: &str = "marker-install-id-zz";
    let anywhere = |rows: &[Row]| {
        rows.iter()
            .any(|r| r.fields.values().any(|v| v.contains(INSTALL)))
    };

    let summary = DevSessionRequest {
        install_id: INSTALL.into(),
        launcher_version: "007.01.000".into(),
        ..summary_request()
    };
    let rows = capture(|| {
        mint(&Tables::new(), summary).unwrap();
    });
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let row = &rows[0];
    assert_eq!(row.level, tracing::Level::INFO);
    let keys: Vec<&str> = row.fields.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "exp",
            "launcher_version",
            "message",
            "session_id",
            "session_kind"
        ]
    );
    assert_eq!(row.fields["session_kind"], "launcher_summary");
    assert_eq!(row.fields["launcher_version"], "7.1.0");
    assert_eq!(row.fields["message"], "Minted launcher-summary token");
    assert!(!anywhere(&rows), "{rows:#?}");

    let player = DevSessionRequest {
        install_id: INSTALL.into(),
        ..request("unused")
    };
    let rows = capture(|| {
        mint(&Tables::new(), player).unwrap();
    });
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert!(
        anywhere(&rows),
        "the capture must see a player's install_id"
    );
}

/// Summary and player mints spend separate per-address allowances: using
/// one up leaves the other working, in both directions.
///
/// Charging the summary arm to `mint_ip` fails the first half; charging a
/// player mint to `mint_summary_ip` fails the second.
#[test]
fn summary_and_player_mints_do_not_share_the_per_ip_quota() {
    let _g = EnvGuard::install();
    let p = policy(2, 100, 10);
    let now = Instant::now();
    let peer = ip("198.51.100.9");
    let player = |i: u32| request(&format!("install-{i}"));

    let t = Tables::new();
    for i in 0..2 {
        mint_inner(&t, &p, peer, summary_request(), now, NOW_UNIX)
            .unwrap_or_else(|e| panic!("summary mint {i} should pass, got {e}"));
    }
    let err = mint_inner(&t, &p, peer, summary_request(), now, NOW_UNIX).unwrap_err();
    match &err {
        AuthError::QuotaExceeded(q) => assert_eq!(q.scope, "mint/summary_ip"),
        other => panic!("expected the summary quota, got {other:?}"),
    }
    assert_eq!(status(err), axum::http::StatusCode::TOO_MANY_REQUESTS);
    mint_inner(&t, &p, peer, player(0), now, NOW_UNIX)
        .expect("a player mint from the same address is unaffected");

    let t = Tables::new();
    for i in 0..2 {
        mint_inner(&t, &p, peer, player(i), now, NOW_UNIX)
            .unwrap_or_else(|e| panic!("player mint {i} should pass, got {e}"));
    }
    let err = mint_inner(&t, &p, peer, player(2), now, NOW_UNIX).unwrap_err();
    match &err {
        AuthError::QuotaExceeded(q) => assert_eq!(q.scope, "mint/ip"),
        other => panic!("expected the player quota, got {other:?}"),
    }
    mint_inner(&t, &p, peer, summary_request(), now, NOW_UNIX)
        .expect("a summary mint from the same address is unaffected");
}

/// A summary mint is charged before its fields are looked at, so a
/// malformed one spends the address's allowance too. With an allowance of
/// one: a malformed summary mint is a 400, and the golden body from the
/// same address is then over quota. The control is the golden body alone
/// on fresh tables, which mints.
///
/// Moving the charge in `mint_summary` below any of the field checks lets
/// that address send malformed summary mints without limit, and fails here:
/// the golden body would mint.
#[test]
fn a_malformed_summary_mint_spends_the_per_ip_allowance() {
    let _g = EnvGuard::install();
    let p = policy(1, 100, 10);
    let now = Instant::now();
    let peer = ip("198.51.100.9");

    mint_inner(&Tables::new(), &p, peer, summary_request(), now, NOW_UNIX)
        .expect("control: the golden body mints on fresh tables");

    type Mutation = fn(&mut DevSessionRequest);
    let malformed: [(&str, Mutation); 4] = [
        ("install_id", |r| r.install_id = "not a token".into()),
        ("machine_id", |r| r.machine_id = "DESKTOP-1".into()),
        ("tags", |r| r.tags = vec!["beta".into()]),
        ("launcher_version", |r| r.launcher_version = "x".into()),
    ];
    for (field, mutate) in malformed {
        let t = Tables::new();
        let mut req = summary_request();
        mutate(&mut req);
        let err = mint_inner(&t, &p, peer, req, now, NOW_UNIX).unwrap_err();
        assert_eq!(status(err), axum::http::StatusCode::BAD_REQUEST, "{field}");

        let err = mint_inner(&t, &p, peer, summary_request(), now, NOW_UNIX).unwrap_err();
        match &err {
            AuthError::QuotaExceeded(q) => assert_eq!(q.scope, "mint/summary_ip", "{field}"),
            other => panic!("{field}: expected the summary quota, got {other:?}"),
        }
        assert_eq!(
            status(err),
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "{field}"
        );
    }
}

/// A summary mint is not charged per `install_id`: the exporter sends a
/// fresh random one each time, so that table would only fill with noise.
/// The control is a player mint, which is refused on its second use of one
/// `install_id` under the same policy.
#[test]
fn a_summary_mint_does_not_charge_the_per_install_quota() {
    let _g = EnvGuard::install();
    let p = policy(100, 1, 10);
    let now = Instant::now();
    let peer = ip("198.51.100.9");

    let t = Tables::new();
    for i in 0..3 {
        mint_inner(&t, &p, peer, summary_request(), now, NOW_UNIX)
            .unwrap_or_else(|e| panic!("summary mint {i} should pass, got {e}"));
    }
    // And those mints left the player's per-install bucket for that id
    // untouched.
    let same_id = || DevSessionRequest {
        install_id: FIXTURE_INSTALL_ID.into(),
        ..request("unused")
    };
    mint_inner(&t, &p, peer, same_id(), now, NOW_UNIX)
        .expect("the first player mint for that install_id");
    let err = mint_inner(&t, &p, peer, same_id(), now, NOW_UNIX).unwrap_err();
    match &err {
        AuthError::QuotaExceeded(q) => assert_eq!(q.scope, "mint/install_id"),
        other => panic!("expected the per-install quota, got {other:?}"),
    }
}

/// A kind that only resembles the summary one is still an unknown kind: it
/// is refused, not minted as a player session.
#[test]
fn a_near_miss_summary_kind_is_refused() {
    let _g = EnvGuard::install();
    for kind in ["launcher_summary ", "Launcher_Summary", "launcher-summary"] {
        let req = DevSessionRequest {
            session_kind: Some(kind.into()),
            ..summary_request()
        };
        let err = mint(&Tables::new(), req).unwrap_err();
        assert!(
            matches!(
                err,
                AuthError::BadField {
                    field: "session_kind",
                    ..
                }
            ),
            "{kind:?}: {err:?}"
        );
    }
}
