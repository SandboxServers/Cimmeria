//! The chunk budgets (body, expansion, rows) and the priority allowance.

use std::time::Instant;

use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::routes::telemetry::chunk::chunk_inner;
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::session_budget::SessionLedger;
use crate::routes::telemetry::upload_gate::{UploadLimits, UploadPolicy, UploadState};
use crate::routes::telemetry::MAX_CHUNK_DECOMPRESSED_BYTES;

use super::{chunk_request, gzip, run, small_chunk, Env, PEER};

/// **A gzip chunk expanding past the cap is refused**, with the production
/// cap. The payload is a fixed 9 MiB of blank lines (a few KiB gzipped),
/// which would parse to an empty chunk and be accepted if the expansion
/// were unbounded or bounded as loosely as it used to be (256 MiB).
#[test]
fn a_chunk_expanding_past_the_cap_is_refused() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let expanded = vec![b'\n'; 9 * 1024 * 1024];
    assert!(expanded.len() as u64 > MAX_CHUNK_DECOMPRESSED_BYTES);
    let err = run(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-bomb", gzip(&expanded)),
        Instant::now(),
    ))
    .unwrap_err();
    assert!(
        matches!(
            err,
            IngestError::OverBudget {
                what: "decompressed bytes",
                limit: MAX_CHUNK_DECOMPRESSED_BYTES
            }
        ),
        "{err:?}"
    );
    assert_eq!(err.into_response().status(), StatusCode::PAYLOAD_TOO_LARGE);
}

/// A chunk with more rows than the cap is refused at the first row past
/// it, and none of it is replayed.
#[test]
fn a_chunk_over_the_row_cap_is_refused() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits {
        chunk_rows: 5,
        ..UploadLimits::default()
    });
    let policy = UploadPolicy::defaults();
    let ok = run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-rows", small_chunk(5)),
        Instant::now(),
    ))
    .unwrap();
    assert_eq!(ok.accepted, 5);
    let err = run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-rows", small_chunk(6)),
        Instant::now(),
    ))
    .unwrap_err();
    assert!(
        matches!(err, IngestError::OverBudget { what: "rows", .. }),
        "{err:?}"
    );
}

/// The compressed body is capped as it streams in.
#[test]
fn a_chunk_body_over_the_cap_is_refused() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits {
        chunk_body_bytes: 64,
        ..UploadLimits::default()
    });
    let err = run(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-body", vec![0u8; 65]),
        Instant::now(),
    ))
    .unwrap_err();
    assert!(matches!(err, IngestError::TooLarge(_, 64)), "{err:?}");
}

// ---- Priority allowance ----------------------------------------------

/// **Priority rows past their allowance are suppressed.** Past the window's
/// budget a priority row spends the priority allowance; past that it is
/// suppressed like any other, and a new window restores both.
#[test]
fn priority_rows_beyond_their_allowance_are_dropped() {
    let mut l = SessionLedger::new(2, 60, 16).with_priority_allowance(3);
    let admitted = (0..10).filter(|_| l.admit("s", true, 0)).count();
    assert_eq!(admitted, 5, "the budget of 2 plus the allowance of 3");
    let t = l.totals("s").unwrap();
    assert_eq!((t.accepted_total, t.suppressed_total), (5, 5));
    assert!(!l.admit("s", true, 59));
    assert!(
        l.admit("s", true, 60),
        "a new window restores the allowance"
    );
}
