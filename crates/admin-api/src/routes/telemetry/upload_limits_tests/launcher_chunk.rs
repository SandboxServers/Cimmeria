//! `POST /api/telemetry/upload-chunk` under a launcher token (AU-15): the
//! launcher limits, the row filter, and the launcher's own budget, each
//! checked against the player path on the same body.

use std::time::Instant;

use crate::routes::dev_session::AuthError;
use crate::routes::telemetry::chunk::chunk_inner_with;
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::launcher_budget::LauncherBudget;
use crate::routes::telemetry::upload_gate::{UploadLimits, UploadPolicy, UploadState};

use super::{chunk_request, gzip, launcher_chunk_request, run, Env, PEER};

/// A key dump row: a known event, so it is never a bad row on the player path.
const KEY_DUMP: &str =
    r#"{"type":"key_dump","ts_ms":1,"seq":3,"source_file":"a.log","key_b64":"AAAA"}"#;

/// One launcher-shaped `client_native` row with `target` and one field.
fn native_row(seq: usize, target: &str, step: &str) -> String {
    format!(
        r#"{{"type":"client_native","ts_ms":1,"seq":{seq},"target":"{target}","level":"info","fields":{{"step":"{step}"}}}}"#
    )
}

/// The adoption row the launcher posts.
fn adoption_row(seq: usize) -> String {
    native_row(seq, "client.launcher.adoption", "start")
}

/// `rows` joined as NDJSON, one per line.
fn ndjson(rows: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for row in rows {
        out.extend_from_slice(row.as_bytes());
        out.push(b'\n');
    }
    out
}

/// Bytes a gzip cannot shrink: a xorshift stream.
fn incompressible(len: usize) -> Vec<u8> {
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

/// **A launcher chunk keeps only its launcher rows.** Of three rows, the
/// adoption row is kept; the same row under another target and the key dump
/// are dropped and counted as bad. Without the filter all three are replayed.
#[test]
fn a_launcher_chunk_keeps_only_launcher_rows() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let budget = LauncherBudget::new(300, 3_000);
    let body = ndjson(&[
        adoption_row(1),
        native_row(2, "client.native.status", "start"),
        KEY_DUMP.to_string(),
    ]);
    let resp = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        launcher_chunk_request("sess-launcher-filter", gzip(&body)),
        Instant::now(),
        &budget,
    ))
    .expect("the launcher chunk is accepted with its bad rows counted");
    assert_eq!((resp.accepted, resp.bad_rows), (1, 2));
}

/// **The same rows on the player path are not filtered.** The player keeps
/// every row it can parse: no bad rows, and the key dump is accepted.
#[test]
fn a_player_chunk_with_the_same_rows_is_not_filtered() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let body = ndjson(&[
        adoption_row(1),
        native_row(2, "client.native.status", "start"),
        KEY_DUMP.to_string(),
    ]);
    let resp = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-player-unfiltered", gzip(&body)),
        Instant::now(),
        &LauncherBudget::new(300, 3_000),
    ))
    .expect("the player chunk is accepted");
    assert_eq!(resp.bad_rows, 0);
    assert_eq!(resp.accepted, 3);
}

/// **A launcher chunk over 64 KiB is refused.** The body is incompressible
/// and gzips past 65 536 bytes: refused as too large under a launcher
/// token, and not refused for size under a player token.
#[test]
fn a_launcher_chunk_over_64_kib_is_refused() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let body = gzip(&incompressible(70_000));
    assert!(body.len() > 65_536, "the body is {} bytes", body.len());
    let err = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        launcher_chunk_request("sess-launcher-big", body.clone()),
        Instant::now(),
        &LauncherBudget::new(300, 3_000),
    ))
    .expect_err("a launcher chunk over 64 KiB is refused");
    assert!(matches!(err, IngestError::TooLarge(..)), "{err:?}");

    let player = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-player-big", body),
        Instant::now(),
        &LauncherBudget::new(300, 3_000),
    ));
    assert!(
        !matches!(player, Err(IngestError::TooLarge(..))),
        "a player chunk of the same bytes is not refused for size"
    );
}

/// **A launcher gzip bomb is cut at 256 KiB.** About 300 KiB of adoption
/// rows gzips well under 64 KiB; the launcher path cuts it past its
/// decompressed cap, as the player path cuts past 8 MiB, and reports
/// `truncated`. The same body under a player token is accepted whole.
#[test]
fn a_launcher_gzip_bomb_is_cut_at_256_kib() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let step = "s".repeat(1_400);
    let rows: Vec<String> = (0..200)
        .map(|seq| native_row(seq, "client.launcher.adoption", &step))
        .collect();
    let body = gzip(&ndjson(&rows));
    assert!(body.len() < 65_536, "the body is {} bytes", body.len());

    let resp = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        launcher_chunk_request("sess-launcher-bomb", body.clone()),
        Instant::now(),
        &LauncherBudget::new(300, 3_000),
    ))
    .expect("an over-budget launcher expansion is truncated, not refused");
    assert!(resp.truncated);
    assert!(resp.parsed_lines < 200, "{} rows parsed", resp.parsed_lines);

    let player = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-player-bomb", body),
        Instant::now(),
        &LauncherBudget::new(300, 3_000),
    ))
    .expect("the player chunk is accepted");
    assert!(!player.truncated);
    assert_eq!(player.parsed_lines, 200);
}

/// **A launcher session has its own row budget.** With a budget of 3 rows
/// per session, a 5-row launcher chunk is cut to 3 accepted and 2
/// suppressed. A player chunk of the same session id is not touched by it.
#[test]
fn launcher_rows_have_their_own_session_budget() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let budget = LauncherBudget::new(3, 1_000);
    let rows: Vec<String> = (1..=5).map(adoption_row).collect();
    let resp = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        launcher_chunk_request("sess-own-budget", gzip(&ndjson(&rows))),
        Instant::now(),
        &budget,
    ))
    .expect("the launcher chunk is accepted");
    assert_eq!((resp.accepted, resp.suppressed), (3, 2));

    let player = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request(
            "sess-own-budget",
            gzip(&ndjson(&[
                native_row(1, "client.native.status", "a"),
                native_row(2, "client.native.status", "b"),
                native_row(3, "client.native.status", "c"),
                native_row(4, "client.native.status", "d"),
                native_row(5, "client.native.status", "e"),
            ])),
        ),
        Instant::now(),
        &budget,
    ))
    .expect("the player chunk is accepted");
    assert_eq!(player.accepted, 5);
}

/// **The launcher's global per-minute ceiling refuses the next chunk.** With
/// 4 rows a minute for all launchers, two sessions of 3 rows each: the first
/// is accepted, the second is refused with the rows scope and replays nothing.
#[test]
fn the_global_launcher_ceiling_refuses_the_next_chunk() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let budget = LauncherBudget::new(1_000, 4);
    let rows: Vec<String> = (1..=3).map(adoption_row).collect();

    let first = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        launcher_chunk_request("sess-ceiling-a", gzip(&ndjson(&rows))),
        Instant::now(),
        &budget,
    ))
    .expect("the first launcher chunk fits the minute");
    assert_eq!(first.accepted, 3);

    let err = run(chunk_inner_with(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        launcher_chunk_request("sess-ceiling-b", gzip(&ndjson(&rows))),
        Instant::now(),
        &budget,
    ))
    .expect_err("the second launcher chunk is over the minute's ceiling");
    match err {
        IngestError::Auth(AuthError::QuotaExceeded(q)) => {
            assert_eq!(q.scope, "upload/launcher_rows");
        }
        other => panic!("expected a rows quota refusal, got {other:?}"),
    }
}
