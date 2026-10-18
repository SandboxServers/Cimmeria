//! TG-PIPE-04: the uploading DLL's process id (`X-Cimmeria-Client-Pid`) is
//! stamped on every replayed row as `client_pid`. An upload without the
//! header, from a DLL that predates it, leaves the attribute out; it is
//! never 0. Driven through the chunk handler with a real request, and
//! captured with the replay tests' field-recording layer.

use std::time::Instant;

use axum::body::Body;
use axum::extract::Request;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, HeaderValue};

use super::chunk::{chunk_inner, client_sent_ms_header, CLIENT_PID_HEADER, CLIENT_SENT_MS_HEADER};
use super::replay_tests::{capture_async, Row};
use super::upload_gate::{UploadLimits, UploadPolicy, UploadState};
use super::upload_limits_tests::{gzip, run, token, Env, PEER};

/// One client log row, as the DLL uploads it.
const ROW: &str = r#"{"type":"client_log","ts_ms":1,"seq":0,"source_file":"a.log","level":"info","category":"raw","message":"m"}"#;

/// Upload one chunk of one row from session `sid`, with `pid` as the header
/// if given. Returns the rows the replay emitted as `launcher.client_log`.
fn replay_one(sid: &str, pid: Option<&str>) -> Vec<Row> {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let mut builder = Request::builder()
        .method("POST")
        .uri("/upload-chunk")
        .header(AUTHORIZATION, format!("Bearer {}", token(sid)));
    if let Some(pid) = pid {
        builder = builder.header(CLIENT_PID_HEADER, pid);
    }
    let request = builder.body(Body::from(gzip(ROW.as_bytes()))).unwrap();
    let (resp, rows) = run(capture_async(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        request,
        Instant::now(),
    )));
    let resp = resp.expect("the chunk is accepted with or without the header");
    assert_eq!(resp.parsed_lines, 1);
    rows.into_iter()
        .filter(|r| r.target == "launcher.client_log")
        .collect()
}

/// **A header's pid is on the replayed row.** The row is the only one the
/// chunk replays, and it carries the pid the DLL sent.
#[test]
fn every_replayed_row_carries_the_uploading_pid() {
    let rows = replay_one("sess-pid-stamp", Some("4242"));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fields["client_pid"], "4242");
}

/// **No header, no attribute.** An older DLL's row carries no `client_pid`
/// at all, not a zero.
#[test]
fn an_upload_without_the_header_has_no_client_pid() {
    let rows = replay_one("sess-pid-absent", None);
    assert_eq!(rows.len(), 1);
    assert!(
        !rows[0].fields.contains_key("client_pid"),
        "{:?}",
        rows[0].fields
    );
}

/// The send stamp parses when it is a positive number, and reads as absent
/// otherwise, so a bad stamp never refuses a chunk.
#[test]
fn sent_ms_header_parses_or_reads_none() {
    let read = |v: Option<&'static str>| {
        let mut headers = HeaderMap::new();
        if let Some(v) = v {
            headers.insert(CLIENT_SENT_MS_HEADER, HeaderValue::from_static(v));
        }
        client_sent_ms_header(&headers)
    };
    assert_eq!(read(Some("1700000000123")), Some(1_700_000_000_123));
    assert_eq!(read(None), None);
    assert_eq!(read(Some("abc")), None);
    assert_eq!(read(Some("-5")), None);
    assert_eq!(read(Some("0")), None);
}

/// **A bad header is absent too.** The chunk is still accepted, and the row
/// is not stamped with a made-up pid.
#[test]
fn an_unreadable_header_is_absent_not_zero() {
    for bad in ["", "not-a-pid", "-1", "4294967296"] {
        let rows = replay_one("sess-pid-bad", Some(bad));
        assert_eq!(rows.len(), 1, "header {bad:?}");
        assert!(
            !rows[0].fields.contains_key("client_pid"),
            "header {bad:?}: {:?}",
            rows[0].fields
        );
    }
}
