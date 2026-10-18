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

/// Upload one camera row with client time `ts_ms` from session `sid`, with
/// `sent` as the send-stamp header if given, and return the `clock_offset_ms`
/// the replayed row carries, with the server's unix ms before and after.
fn camera_offset(sid: &str, ts_ms: i64, sent: Option<i64>) -> (i64, i64, i64) {
    let now_ms = || {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        i64::try_from(d.as_millis()).unwrap()
    };
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let row = format!(
        r#"{{"type":"client_native","ts_ms":{ts_ms},"seq":0,"target":"client.camera.at_interact","level":"info","fields":{{"zoom":1.0}}}}"#
    );
    let mut builder = Request::builder()
        .method("POST")
        .uri("/upload-chunk")
        .header(AUTHORIZATION, format!("Bearer {}", token(sid)));
    if let Some(sent) = sent {
        builder = builder.header(CLIENT_SENT_MS_HEADER, sent.to_string());
    }
    let request = builder.body(Body::from(gzip(row.as_bytes()))).unwrap();
    let before = now_ms();
    let (resp, rows) = run(capture_async(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        request,
        Instant::now(),
    )));
    let after = now_ms();
    resp.expect("the chunk is accepted");
    let row = rows
        .into_iter()
        .find(|r| r.target == "client.native")
        .expect("the camera row replays");
    (
        row.fields["clock_offset_ms"].parse().unwrap(),
        before,
        after,
    )
}

/// **The header reaches the clock.** A stamp 50 s after the row's own time
/// makes the offset sample `recv - stamp`, not `recv - ts_ms`; without the
/// header the sample is the old one. Dropping the wiring in `chunk_flow`
/// fails the first half.
#[test]
fn the_send_stamp_header_sets_the_clock_offset() {
    let ts_ms = 1_000_000;
    let sent = ts_ms + 50_000;
    let (offset, before, after) = camera_offset("sess-clock-stamped", ts_ms, Some(sent));
    assert!(
        (before - sent..=after - sent).contains(&offset),
        "offset {offset} is not recv - stamp"
    );
    let (offset, before, after) = camera_offset("sess-clock-unstamped", ts_ms, None);
    assert!(
        (before - ts_ms..=after - ts_ms).contains(&offset),
        "offset {offset} is not recv - ts_ms"
    );
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
