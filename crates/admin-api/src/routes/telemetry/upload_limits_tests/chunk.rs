//! The chunk budgets (body, expansion, rows) and the priority allowance.

use std::time::Instant;

use crate::routes::telemetry::chunk::{chunk_inner, inflate_bounded, parse_rows_bounded};
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::dto::TelemetryEvent;
use crate::routes::telemetry::field_caps::marker;
use crate::routes::telemetry::session_budget::SessionLedger;
use crate::routes::telemetry::upload_gate::{UploadLimits, UploadPolicy, UploadState};
use crate::routes::telemetry::{MAX_CHUNK_DECOMPRESSED_BYTES, MAX_CHUNK_ROW_BYTES};

use super::{chunk_request, gzip, run, small_chunk, Env, PEER};

/// `rows` client log rows of about `message_len` bytes each, as NDJSON.
fn rows_of(rows: usize, message_len: usize) -> Vec<u8> {
    let message = "m".repeat(message_len);
    let mut out = Vec::new();
    for seq in 0..rows {
        out.extend_from_slice(
            format!(
                r#"{{"type":"client_log","ts_ms":1,"seq":{seq},"source_file":"a.log","level":"info","category":"raw","message":"{message}"}}"#
            )
            .as_bytes(),
        );
        out.push(b'\n');
    }
    out
}

/// **A chunk expanding past the cap is truncated, not refused.** With the
/// production cap, a fixed ~9.3 MiB of rows (4,500 rows of ~2 KiB, under
/// the row cap) is cut at the last whole row within 8 MiB: those rows are
/// replayed, the rest dropped, and the answer is a 200 with `truncated`.
/// Refusing it would make the deployed launcher retry the same backlog
/// forever; not cutting it (no cap, or the old 256 MiB one) would answer
/// without `truncated`.
#[test]
fn a_chunk_expanding_past_the_cap_is_truncated() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let ndjson = rows_of(4_500, 2_000);
    assert!(ndjson.len() as u64 > MAX_CHUNK_DECOMPRESSED_BYTES);
    let resp = run(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-expand-cut", gzip(&ndjson)),
        Instant::now(),
    ))
    .expect("an over-budget expansion is truncated, not refused");
    assert!(resp.truncated);
    // Rows differ by a few bytes (the `seq` digits): the whole rows that
    // fit in 8 MiB, give or take a few.
    let row_len = ndjson.len() as u64 / 4_500;
    let fit = MAX_CHUNK_DECOMPRESSED_BYTES / row_len;
    assert!(
        (fit - 3..=fit + 3).contains(&resp.parsed_lines),
        "{} rows parsed, about {fit} fit",
        resp.parsed_lines
    );
    assert_eq!(resp.accepted, resp.parsed_lines);
}

/// A gzip bomb of blank lines is cut at the cap too: nothing to replay,
/// and no more than the cap is ever expanded.
#[test]
fn a_blank_line_bomb_is_cut_at_the_cap() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let resp = run(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-bomb", gzip(&vec![b'\n'; 9 * 1024 * 1024])),
        Instant::now(),
    ))
    .unwrap();
    assert!(resp.truncated);
    assert_eq!(resp.parsed_lines, 0);
}

/// The cut keeps whole lines only.
#[test]
fn the_expansion_cut_lands_after_the_last_whole_line() {
    let inflated = inflate_bounded(&gzip(b"aaa\nbbbb\ncc"), 7).unwrap();
    assert_eq!(inflated.bytes, b"aaa\n");
    assert!(inflated.cut.is_some());
    let whole = inflate_bounded(&gzip(b"aaa\n"), 7).unwrap();
    assert_eq!((whole.bytes.as_slice(), whole.cut), (&b"aaa\n"[..], None));
}

/// **A row the server cannot parse is skipped, not fatal.** A chunk with a
/// malformed row, a row of an unknown event type and a row that is not
/// UTF-8 replays its good rows and counts the three bad ones. Refusing it
/// would make the uploader re-send the same chunk forever.
#[test]
fn rows_that_do_not_parse_are_skipped_and_counted() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let mut ndjson = rows_of(2, 4);
    ndjson.extend_from_slice(b"{not json\n");
    ndjson.extend_from_slice(b"{\"type\":\"from_the_future\",\"x\":1}\n");
    ndjson.extend_from_slice(b"\xff\xfe\n");
    ndjson.extend_from_slice(&rows_of(1, 4));
    let resp = run(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-bad-rows", gzip(&ndjson)),
        Instant::now(),
    ))
    .expect("bad rows are skipped, the chunk is accepted");
    assert_eq!((resp.accepted, resp.bad_rows), (3, 3));
}

/// **A row over the per-row cap is counted bad without being parsed.** A
/// well-formed row of a fixed ~66,000 bytes, just over the 64 KiB cap, is
/// not replayed and counts as one bad row; parsing it would have accepted
/// it. A row just under the cap is replayed.
#[test]
fn a_row_over_the_row_cap_is_not_parsed() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let over = rows_of(1, 66_000);
    assert!(over.len() > MAX_CHUNK_ROW_BYTES && over.len() < 70_000);
    let resp = run(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-big-row", gzip(&over)),
        Instant::now(),
    ))
    .unwrap();
    assert_eq!((resp.accepted, resp.bad_rows), (0, 1));

    let under = rows_of(1, 60_000);
    let resp = run(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-big-row", gzip(&under)),
        Instant::now(),
    ))
    .unwrap();
    assert_eq!((resp.accepted, resp.bad_rows), (1, 0));
}

/// Parsed rows come out with their strings already capped, so a chunk
/// never holds more than one uncapped row.
#[test]
fn parsed_rows_are_capped_as_they_parse() {
    let rows = parse_rows_bounded(&rows_of(2, 10_000), 10, MAX_CHUNK_ROW_BYTES);
    assert_eq!((rows.events.len(), rows.bad), (2, 0));
    for ev in rows.events {
        let TelemetryEvent::ClientLog(e) = ev else {
            panic!("expected a client log row")
        };
        assert!(
            e.message.ends_with(&marker(10_000)),
            "message was not capped"
        );
    }
}

/// A body that is not gzip at all is still refused (400).
#[test]
fn a_body_that_is_not_gzip_is_refused() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let err = run(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-not-gzip", b"plain text".to_vec()),
        Instant::now(),
    ))
    .unwrap_err();
    assert!(matches!(err, IngestError::Gzip(_)), "{err:?}");
}

/// A chunk with more rows than the cap replays the first ones and says it
/// was truncated; one at the cap is not.
#[test]
fn a_chunk_over_the_row_cap_replays_the_first_rows() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits {
        chunk_rows: 5,
        ..UploadLimits::default()
    });
    let policy = UploadPolicy::defaults();
    let at_cap = run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-rows", small_chunk(5)),
        Instant::now(),
    ))
    .unwrap();
    assert_eq!((at_cap.accepted, at_cap.truncated), (5, false));
    let over = run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-rows", small_chunk(8)),
        Instant::now(),
    ))
    .unwrap();
    assert_eq!(
        (over.accepted, over.parsed_lines, over.truncated),
        (5, 5, true)
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
