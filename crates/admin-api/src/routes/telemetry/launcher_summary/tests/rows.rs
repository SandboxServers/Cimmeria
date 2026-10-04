//! The shape of the three rows: their targets, level, discriminators and
//! exact key sets. A dashboard filters and groups on these names, so a
//! field added, renamed or dropped has to show up here.

use serde_json::json;

use crate::routes::dev_session::{decode_token, load_secret};

use super::super::rows::{duration_bucket, LAUNCHER_SUMMARY_BATCH_TARGET, LAUNCHER_SUMMARY_TARGET};
use super::{batch, batch_rows, capture, element, id, phase_rows, summary_rows, Env, Harness};

/// A summary carrying every optional field writes a row with exactly these
/// keys, at INFO, on the summary target, with `event = launcher_summary`.
#[test]
fn a_full_summary_row_has_exactly_these_keys() {
    let _env = Env::install();
    let h = Harness::new();
    let (_, rows) = capture(|| h.verdicts(&batch(vec![element(1)])));
    let row = summary_rows(&rows)[0];
    assert_eq!(row.target, "launcher.summary");
    assert_eq!(row.target, LAUNCHER_SUMMARY_TARGET);
    assert_eq!(row.level, tracing::Level::INFO);
    assert_eq!(
        row.keys(),
        [
            "arch",
            "attempt_id",
            "cimmeria.session_kind",
            "duration_bucket",
            "duration_ms",
            "error_code",
            "event",
            "event_id",
            "launcher_version",
            "message",
            "operation",
            "os",
            "outcome",
            "phase",
            "retry_count",
            "schema_version",
        ]
    );
    let f = &row.fields;
    assert_eq!(f["event"], "launcher_summary");
    assert_eq!(f["event_id"], id(0xe5, 1));
    assert_eq!(f["attempt_id"], id(0xa5, 1));
    assert_eq!(f["operation"], "install");
    assert_eq!(f["phase"], "download");
    assert_eq!(f["outcome"], "failed");
    assert_eq!(f["error_code"], "install_failed");
    assert_eq!(f["duration_ms"], "81234");
    assert_eq!(f["duration_bucket"], "lt_5m");
    assert_eq!(f["retry_count"], "0");
    assert_eq!(f["launcher_version"], "0.1.0");
    assert_eq!(f["os"], "windows");
    assert_eq!(f["arch"], "x86_64");
    assert_eq!(f["schema_version"], "1");
    assert_eq!(f["cimmeria.session_kind"], "launcher_summary");
    assert_eq!(f["message"], "launcher summary");
}

/// An absent optional is an absent field, never a sentinel: a summary with
/// no `error_code`, no `duration_ms` and no `phases` writes a row without
/// those keys (and without a bucket), and no phase row.
#[test]
fn an_absent_optional_is_an_absent_field() {
    let _env = Env::install();
    let bare = json!({
        "event_id": id(0xe5, 1),
        "attempt_id": id(0xa5, 1),
        "operation": "launch",
        "phase": "running",
        "outcome": "unknown",
        "retry_count": 3,
        "launcher_version": "007.01.000",
        "os": "macos",
        "arch": "aarch64",
    });
    let h = Harness::new();
    let (_, rows) = capture(|| h.verdicts(&batch(vec![bare])));
    let row = summary_rows(&rows)[0];
    assert_eq!(
        row.keys(),
        [
            "arch",
            "attempt_id",
            "cimmeria.session_kind",
            "event",
            "event_id",
            "launcher_version",
            "message",
            "operation",
            "os",
            "outcome",
            "phase",
            "retry_count",
            "schema_version",
        ]
    );
    assert_eq!(row.fields["outcome"], "unknown");
    // The version is written from its parsed integers, not as it was sent.
    assert_eq!(row.fields["launcher_version"], "7.1.0");
    assert!(phase_rows(&rows).is_empty());
}

/// Each `phases` entry of an accepted summary writes one row with exactly
/// these keys, on the summary target, under its own `event` so a count of
/// `launcher_summary` rows stays a count of attempts.
#[test]
fn a_phase_row_has_exactly_these_keys() {
    let _env = Env::install();
    let h = Harness::new();
    let (_, rows) = capture(|| h.verdicts(&batch(vec![element(1)])));
    let phases = phase_rows(&rows);
    assert_eq!(phases.len(), 2);
    for row in &phases {
        assert_eq!(row.target, "launcher.summary");
        assert_eq!(row.level, tracing::Level::INFO);
        assert_eq!(
            row.keys(),
            [
                "arch",
                "attempt_id",
                "cimmeria.session_kind",
                "duration_bucket",
                "duration_ms",
                "event",
                "launcher_version",
                "message",
                "operation",
                "os",
                "phase",
                "schema_version",
            ]
        );
        assert_eq!(row.fields["event"], "launcher_phase");
        assert_eq!(row.fields["attempt_id"], id(0xa5, 1));
        assert_eq!(row.fields["operation"], "install");
        assert_eq!(row.fields["cimmeria.session_kind"], "launcher_summary");
        assert_eq!(row.fields["message"], "launcher phase");
    }
    let timing = |i: usize| {
        let f = &phases[i].fields;
        (&*f["phase"], &*f["duration_ms"], &*f["duration_bucket"])
    };
    assert_eq!(timing(0), ("starting", "12", "lt_1s"));
    assert_eq!(timing(1), ("download", "81000", "lt_5m"));
    assert_eq!(summary_rows(&rows).len(), 1, "the attempt is counted once");
}

/// A duplicate and a rejected summary write no phase rows: only an
/// accepted summary's phases are emitted.
#[test]
fn only_an_accepted_summary_emits_phase_rows() {
    let _env = Env::install();
    let mut invalid = element(2);
    invalid["os"] = "freebsd".into();
    let h = Harness::new();
    let (_, rows) = capture(|| h.verdicts(&batch(vec![element(1), element(1), invalid])));
    assert_eq!(phase_rows(&rows).len(), 2);
}

/// Every request that reaches validation writes one batch row with exactly
/// these keys, at INFO, on the ingest target: the server's own counts and
/// the launcher's drop counters.
#[test]
fn the_batch_row_has_exactly_these_keys() {
    let _env = Env::install();
    let mut invalid = element(3);
    invalid["retry_count"] = 101.into();
    let mut body = batch(vec![element(1), element(1), element(2), invalid]);
    body["client_dropped"] = json!({ "overflow": 4, "expired": 5, "rejected": 6 });

    let h = Harness::new();
    let (_, rows) = capture(|| h.verdicts(&body));
    let batches = batch_rows(&rows);
    assert_eq!(batches.len(), 1);
    let row = batches[0];
    assert_eq!(row.target, "launcher.ingest");
    assert_eq!(row.target, LAUNCHER_SUMMARY_BATCH_TARGET);
    assert_eq!(row.level, tracing::Level::INFO);
    assert_eq!(
        row.keys(),
        [
            "accepted",
            "client_dropped_expired",
            "client_dropped_overflow",
            "client_dropped_rejected",
            "duplicate",
            "event",
            "message",
            "rejected",
        ]
    );
    let f = &row.fields;
    assert_eq!(f["event"], "launcher_summary_batch");
    assert_eq!(
        (&*f["accepted"], &*f["duplicate"], &*f["rejected"]),
        ("2", "1", "1")
    );
    assert_eq!(f["client_dropped_overflow"], "4");
    assert_eq!(f["client_dropped_expired"], "5");
    assert_eq!(f["client_dropped_rejected"], "6");
    assert_eq!(f["message"], "launcher summary batch");
}

/// No row says who sent the request: not the token's session id or
/// subject, not the peer address. The token and the peer are the ones the
/// request really carried, so their absence is not an accident of the
/// fixture.
#[test]
fn no_row_carries_the_session_the_subject_or_the_peer() {
    let _env = Env::install();
    let h = Harness::new();
    let raw = h.headers[axum::http::header::AUTHORIZATION]
        .to_str()
        .unwrap();
    let token = raw.strip_prefix("Bearer ").unwrap();
    let claims = decode_token(token, &load_secret().unwrap()).unwrap();
    let peer = h.peer.to_string();

    let (_, rows) = capture(|| h.verdicts(&batch(vec![element(1)])));
    assert_eq!(rows.len(), 4, "summary, two phases, batch: {rows:#?}");
    for row in &rows {
        assert!(!row.mentions(&claims.sid), "session id in {row:#?}");
        assert!(!row.mentions(&peer), "peer address in {row:#?}");
        for key in ["session_id", "install_id", "sub", "peer"] {
            assert!(!row.fields.contains_key(key), "{key} in {row:#?}");
        }
    }
}

/// The bucket edges: each band starts at its lower bound.
#[test]
fn duration_buckets_break_at_their_lower_bounds() {
    for (ms, bucket) in [
        (0, "lt_1s"),
        (999, "lt_1s"),
        (1_000, "lt_10s"),
        (9_999, "lt_10s"),
        (10_000, "lt_1m"),
        (59_999, "lt_1m"),
        (60_000, "lt_5m"),
        (299_999, "lt_5m"),
        (300_000, "lt_30m"),
        (1_799_999, "lt_30m"),
        (1_800_000, "ge_30m"),
        (604_800_000, "ge_30m"),
        (u32::MAX, "ge_30m"),
    ] {
        assert_eq!(duration_bucket(ms), bucket, "{ms} ms");
    }
}
