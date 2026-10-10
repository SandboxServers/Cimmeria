//! The bundle budgets: zip entries, expanded bytes, lines.

use std::time::Instant;

use crate::routes::telemetry::bundle::{bundle_inner, declared_zip_entries, unpack_and_replay};
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::upload_gate::{UploadLimits, UploadPolicy, UploadState};

use super::{bundle_request, claims, run, zip_of, Env, PEER};

/// **A zip of many small entries is refused at the entry cap**, from its
/// end record, before the archive is opened or a line replayed.
#[test]
fn a_zip_with_too_many_entries_is_refused() {
    let names: Vec<String> = (0..1500)
        .map(|i| format!("Binaries/sessions/{i}.log"))
        .collect();
    let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"x\n"[..])).collect();
    let zip = zip_of(&entries);
    assert_eq!(declared_zip_entries(&zip), Some(1500));

    let limits = UploadLimits::default();
    let err = unpack_and_replay(&claims("sess-zip-entries"), &zip, &limits).unwrap_err();
    assert!(
        matches!(
            err,
            IngestError::OverBudget {
                what: "zip entries",
                ..
            }
        ),
        "{err:?}"
    );

    // And through the handler, which checks the end record first.
    let _env = Env::install();
    let state = UploadState::new(limits);
    let err = run(bundle_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        bundle_request("sess-zip-entries", &zip),
        Instant::now(),
    ))
    .unwrap_err();
    assert!(
        matches!(
            err,
            IngestError::OverBudget {
                what: "zip entries",
                ..
            }
        ),
        "{err:?}"
    );
}

/// The expanded bytes of every entry together are capped, on the declared
/// sizes before anything is replayed.
#[test]
fn a_zip_over_the_expanded_byte_cap_is_refused() {
    let data = [b'a'; 60];
    let zip = zip_of(&[("a.log", &data), ("b.log", &data), ("c.log", &data)]);
    let limits = UploadLimits {
        bundle_expanded_bytes: 100,
        ..UploadLimits::default()
    };
    let err = unpack_and_replay(&claims("sess-zip-bytes"), &zip, &limits).unwrap_err();
    assert!(
        matches!(
            err,
            IngestError::OverBudget {
                what: "expanded bytes",
                limit: 100
            }
        ),
        "{err:?}"
    );
}

/// Replay stops at the line budget.
#[test]
fn a_zip_over_the_line_budget_is_refused() {
    let zip = zip_of(&[("a.log", b"1\n2\n3\n4\n5\n")]);
    let limits = UploadLimits {
        bundle_lines: 3,
        ..UploadLimits::default()
    };
    let err = unpack_and_replay(&claims("sess-zip-lines"), &zip, &limits).unwrap_err();
    assert!(
        matches!(
            err,
            IngestError::OverBudget {
                what: "lines",
                limit: 3
            }
        ),
        "{err:?}"
    );
    let ok = unpack_and_replay(
        &claims("sess-zip-lines"),
        &zip_of(&[("a.log", b"1\n2\n\n3\n")]),
        &limits,
    )
    .unwrap();
    assert_eq!(ok, (1, 3));
}

/// A bundle within every budget is accepted through the handler.
#[test]
fn a_small_bundle_is_accepted() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let zip = zip_of(&[("Binaries/sgwdebuglog", b"hello\nworld\n")]);
    let resp = run(bundle_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        bundle_request("sess-bundle-ok", &zip),
        Instant::now(),
    ))
    .unwrap();
    assert_eq!((resp.files, resp.lines), (1, 2));
}
