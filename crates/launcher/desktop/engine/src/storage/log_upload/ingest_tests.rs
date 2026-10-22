use super::*;
use crate::client_setup::login_servers::LoginServer;
use crate::game_telemetry::launcher_session::{BatchOutcome, LauncherSender, LauncherTarget};
use crate::game_telemetry::Identity;
use crate::test_capture::capture;
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CHUNK: &str = "/api/telemetry/upload-chunk";
const UPLOAD_ID: &str = "0123456789abcdef";

fn line(n: u64, message: String) -> LogLine {
    LogLine {
        file: "launcher.2026-10-22.log".into(),
        line_no: n,
        end: n * 100,
        ts_ms: Some(1_700_000_000_000 + n as i64),
        level: Some("warn".into()),
        target: Some("cimmeria_launcher::net".into()),
        message,
        truncated: false,
    }
}

fn rows_of(count: usize, message: impl Fn(usize) -> String) -> Vec<TelemetryEvent> {
    let lines: Vec<LogLine> = (0..count).map(|i| line(i as u64 + 1, message(i))).collect();
    rows(
        &lines,
        LAUNCHER_LOG_TARGET,
        UPLOAD_ID,
        "desktop-1.2.3",
        "windows",
    )
}

/// Pseudo-random lowercase hex: gzip gets about 2:1 at best, never the 10:1 of log text.
fn random_hex(seed: u64, len: usize) -> String {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            char::from_digit((state % 16) as u32, 16).unwrap()
        })
        .collect()
}

// ---- row shape ----

#[test]
fn a_row_carries_the_c_lf5_fields() {
    let mut l = line(7, "connect <removed> refused".into());
    l.truncated = true;
    let out = rows(
        &[l],
        LAUNCHER_LOG_TARGET,
        UPLOAD_ID,
        "desktop-1.2.3",
        "windows",
    );
    let TelemetryEvent::ClientNative(row) = &out[0] else {
        panic!("a log row is a client_native event");
    };
    assert_eq!(
        (row.target.as_str(), row.level.as_str(), row.seq),
        ("client.launcher.log", "info", 0)
    );
    assert_eq!(row.ts_ms, 1_700_000_000_007);
    let f = &row.fields;
    assert_eq!(f["message"], "connect <removed> refused");
    assert_eq!(f["log_level"], "WARN");
    assert_eq!(f["log_target"], "cimmeria_launcher::net");
    assert_eq!(f["file"], "launcher.2026-10-22.log");
    assert_eq!(f["line_no"], 7);
    assert_eq!(f["upload_id"], UPLOAD_ID);
    assert_eq!(f["truncated"], true);
    assert_eq!(f["launcher_version"], "desktop-1.2.3");
    assert_eq!(f["os"], "windows");
    assert_eq!(f.len(), 9, "no key outside C-LF5: {f:?}");
}

#[test]
fn seq_counts_from_zero_and_bad_values_are_cleaned_not_left_to_be_dropped() {
    let mut l = line(0, "m".repeat(3_000));
    l.file = "C:\\Users\\bob\\x.log".into();
    l.level = Some("verbose".into());
    l.target = Some("a b".into());
    let out = rows(&[l.clone(), l], GAME_LOG_TARGET, UPLOAD_ID, "v 1", "Win 10");
    let TelemetryEvent::ClientNative(second) = &out[1] else {
        panic!("client_native");
    };
    assert_eq!(second.seq, 1);
    let f = &second.fields;
    assert_eq!(second.target, "client.launcher.game_log");
    assert_eq!(f["message"].as_str().unwrap().len(), 2_048);
    assert_eq!(f["truncated"], true, "a cut message says so");
    assert!(!f["file"].as_str().unwrap().contains(['\\', ':']));
    assert!(f.get("log_level").is_none(), "an unknown level is omitted");
    assert_eq!(f["log_target"], "a_b");
    assert_eq!(f["line_no"], 1, "line_no is at least 1");
    assert_eq!(f["launcher_version"], "v_1");
    assert_eq!(f["os"], "win_10");
}

#[test]
fn the_target_list_matches_the_shared_file() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../launcher-core/src/telemetry/launcher_log_targets.txt"
    );
    let text = std::fs::read_to_string(path).unwrap();
    assert!(!text.contains('\r'), "the target file must use LF endings");
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, LOG_UPLOAD_TARGETS);
}

// ---- pacing, with a fake clock and a recording sink ----

#[derive(Clone, Default)]
struct Clock(Arc<AtomicU64>);
impl Clock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
    fn advance(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct FakePacer {
    clock: Clock,
    sleeps: Vec<u64>,
}
impl Pacer for FakePacer {
    fn now_ms(&self) -> u64 {
        self.clock.now()
    }
    fn sleep_ms(&mut self, ms: u64) -> impl Future<Output = ()> + Send {
        self.sleeps.push(ms);
        self.clock.advance(ms);
        std::future::ready(())
    }
}

/// Records when each post started on the fake clock, and answers from a script.
struct FakeSink {
    clock: Clock,
    send_ms: u64,
    starts: Vec<u64>,
    /// The post number (1-based) that gets this outcome; the rest are `Posted`.
    script: Vec<(usize, BatchOutcome)>,
}
impl FakeSink {
    fn new(pacer: &FakePacer, send_ms: u64) -> Self {
        Self {
            clock: pacer.clock.clone(),
            send_ms,
            starts: Vec::new(),
            script: Vec::new(),
        }
    }
}
impl ChunkSink for FakeSink {
    fn post(&mut self, _batch: &[TelemetryEvent]) -> impl Future<Output = BatchOutcome> + Send {
        self.starts.push(self.clock.now());
        self.clock.advance(self.send_ms);
        let n = self.starts.len();
        let outcome = self
            .script
            .iter()
            .find(|(at, _)| *at == n)
            .map_or(BatchOutcome::Posted, |(_, o)| *o);
        std::future::ready(outcome)
    }
}

/// `send_with` with a cancel token that never fires.
async fn send_uncancelled(
    sink: &mut FakeSink,
    events: &[TelemetryEvent],
    pacer: &mut FakePacer,
    on_chunk_sent: impl FnMut(usize),
) -> IngestOutcome {
    send_with(
        sink,
        events,
        pacer,
        &CancellationToken::new(),
        on_chunk_sent,
    )
    .await
}

fn forty_chunks() -> Vec<TelemetryEvent> {
    rows_of(40_000, |i| format!("line {i}"))
}

#[tokio::test]
async fn forty_chunks_start_at_least_3500_ms_apart_and_never_18_plus_in_a_minute() {
    let events = forty_chunks();
    let mut pacer = FakePacer::default();
    let mut sink = FakeSink::new(&pacer, 0);
    let mut done = 0usize;
    let out = send_uncancelled(&mut sink, &events, &mut pacer, |n| done += n).await;
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 40_000 });
    assert_eq!(done, 40_000);
    assert_eq!(sink.starts.len(), 40);
    assert_eq!(pacer.sleeps.len(), 39, "no sleep before the first chunk");
    assert!(sink
        .starts
        .windows(2)
        .all(|w| w[1] - w[0] >= MIN_CHUNK_INTERVAL_MS));
    for (i, &start) in sink.starts.iter().enumerate() {
        let in_window = sink.starts[i..]
            .iter()
            .take_while(|&&s| s < start + 60_000)
            .count();
        assert!(in_window <= 18, "{in_window} chunks in 60 s from #{i}");
    }
    assert!(pacer.clock.now() >= 39 * MIN_CHUNK_INTERVAL_MS);
}

#[tokio::test]
async fn a_slow_send_shortens_or_removes_the_next_sleep() {
    let events = rows_of(3_000, |i| format!("line {i}"));
    let mut pacer = FakePacer::default();
    let mut sink = FakeSink::new(&pacer, 5_000);
    send_uncancelled(&mut sink, &events, &mut pacer, |_| {}).await;
    assert!(
        pacer.sleeps.is_empty(),
        "a 5 s send already waited long enough"
    );

    let mut pacer = FakePacer::default();
    let mut sink = FakeSink::new(&pacer, 1_000);
    send_uncancelled(&mut sink, &events, &mut pacer, |_| {}).await;
    assert_eq!(pacer.sleeps, vec![2_500, 2_500]);
}

#[tokio::test]
async fn a_429_on_chunk_21_ends_partial_with_no_sleep_after_it() {
    let events = forty_chunks();
    let mut pacer = FakePacer::default();
    let mut sink = FakeSink::new(&pacer, 0);
    sink.script = vec![(
        21,
        BatchOutcome::Throttled {
            retry_after_secs: 5,
        },
    )];
    let mut done = 0usize;
    let out = send_uncancelled(&mut sink, &events, &mut pacer, |n| done += n).await;
    assert_eq!(
        out,
        IngestOutcome::Partial {
            lines_sent: 20_000,
            retry_after_secs: 60
        },
        "Retry-After 5 still waits the 60 s floor (R16)"
    );
    assert_eq!(done, 20_000);
    assert_eq!(sink.starts.len(), 21, "chunk 22 is never started");
    assert_eq!(
        pacer.sleeps.len(),
        20,
        "one sleep before each of chunks 2..=21"
    );
}

#[tokio::test]
async fn the_first_post_failing_to_mint_is_unreachable_a_later_one_is_failed() {
    let events = rows_of(2_000, |i| format!("line {i}"));
    let mint = BatchOutcome::MintFailed {
        reason: "transport",
    };
    let mut pacer = FakePacer::default();
    let mut sink = FakeSink::new(&pacer, 0);
    sink.script = vec![(1, mint)];
    assert_eq!(
        send_uncancelled(&mut sink, &events, &mut pacer, |_| {}).await,
        IngestOutcome::Unreachable
    );
    let mut pacer = FakePacer::default();
    let mut sink = FakeSink::new(&pacer, 0);
    sink.script = vec![(2, mint)];
    assert_eq!(
        send_uncancelled(&mut sink, &events, &mut pacer, |_| {}).await,
        IngestOutcome::Failed
    );
}

#[test]
fn a_failed_upload_says_why_in_the_finish_event() {
    let events = rows_of(2_000, |i| format!("line {i}"));
    let cases = [
        (
            1,
            BatchOutcome::MintFailed {
                reason: "transport",
            },
            "unreachable",
            "transport",
        ),
        (
            2,
            BatchOutcome::Throttled {
                retry_after_secs: 90,
            },
            "partial",
            "throttled",
        ),
    ];
    for (post, outcome, name, reason) in cases {
        let mut pacer = FakePacer::default();
        let mut sink = FakeSink::new(&pacer, 0);
        sink.script = vec![(post, outcome)];
        // A second live dispatcher keeps the callsite interest from caching "never".
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let (_, logged) = capture(|| {
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(send_uncancelled(&mut sink, &events, &mut pacer, |_| {}))
        });
        let finished = logged
            .iter()
            .find(|e| e.field("event") == Some("log_upload_finished"))
            .expect("a finish event");
        assert_eq!(finished.field("outcome"), Some(name));
        assert_eq!(finished.field("reason"), Some(reason));
    }
}

// ---- the real sender against an HTTP double ----

fn target(url: &str) -> LauncherTarget {
    LauncherTarget {
        identity: Identity {
            install_id: Uuid::from_u128(7),
            machine_id: "0123456789abcdef".into(),
        },
        servers: vec![LoginServer {
            name: "fixture".into(),
            url: url.into(),
        }],
    }
}

/// A mint route that only matches a launcher-kind body with `purpose = "logs"`.
async fn mount_logs_mint(server: &MockServer, times: u64) {
    let expires_at_ms = chrono::Utc::now().timestamp_millis() + 3_600_000;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .and(body_partial_json(
            serde_json::json!({"session_kind": "launcher", "purpose": "logs"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "session_id": "session-1", "token": "payload.sig", "expires_at_ms": expires_at_ms,
            "upload_endpoint": format!("{}/api/telemetry", server.uri()),
            "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
        })))
        .expect(times)
        .mount(server)
        .await;
}

/// Every chunk body the double received, gunzipped: (compressed bytes, rows).
async fn received(server: &MockServer) -> Vec<(usize, Vec<serde_json::Value>)> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.path() == CHUNK)
        .map(|r| {
            let mut text = String::new();
            flate2::read::GzDecoder::new(&r.body[..])
                .read_to_string(&mut text)
                .unwrap();
            let rows = text
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
            (r.body.len(), rows)
        })
        .collect()
}

async fn run(server: &MockServer, events: &[TelemetryEvent]) -> (IngestOutcome, Vec<usize>) {
    let sender = LauncherSender::logs();
    let mut pacer = FakePacer::default();
    let mut chunks = Vec::new();
    let out = send_logs(
        &sender,
        &target(&server.uri()),
        events,
        &mut pacer,
        &CancellationToken::new(),
        |n| {
            chunks.push(n);
        },
    )
    .await;
    (out, chunks)
}

#[tokio::test]
async fn the_mint_asks_for_the_logs_purpose_and_2500_rows_go_as_3_chunks() {
    let server = MockServer::start().await;
    mount_logs_mint(&server, 1).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let (out, chunks) = run(&server, &rows_of(2_500, |i| format!("line {i}"))).await;
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 2_500 });
    assert_eq!(chunks, vec![1_000, 1_000, 500]);
    assert_eq!(received(&server).await.len(), 3);
}

#[tokio::test]
async fn a_429_on_chunk_2_stops_the_upload() {
    let server = MockServer::start().await;
    mount_logs_mint(&server, 1).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "600"))
        .mount(&server)
        .await;
    let (out, chunks) = run(&server, &rows_of(2_500, |i| format!("line {i}"))).await;
    assert_eq!(
        out,
        IngestOutcome::Partial {
            lines_sent: 1_000,
            retry_after_secs: 600
        }
    );
    assert_eq!(chunks, vec![1_000]);
    assert_eq!(received(&server).await.len(), 2, "chunk 3 is not sent");
}

#[tokio::test]
async fn a_rejected_token_mints_once_more_and_a_413_fails_the_upload() {
    let server = MockServer::start().await;
    mount_logs_mint(&server, 2).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(401))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(413))
        .mount(&server)
        .await;
    let (out, chunks) = run(&server, &rows_of(10, |i| format!("line {i}"))).await;
    assert_eq!(out, IngestOutcome::Failed);
    assert!(chunks.is_empty());
}

#[tokio::test]
async fn an_offline_server_is_unreachable() {
    let sender = LauncherSender::logs();
    let mut pacer = FakePacer::default();
    let out = send_logs(
        &sender,
        &target("http://127.0.0.1:9"),
        &rows_of(3, |i| format!("line {i}")),
        &mut pacer,
        &CancellationToken::new(),
        |_| {},
    )
    .await;
    assert_eq!(out, IngestOutcome::Unreachable);
}

#[tokio::test]
async fn large_rows_are_split_under_both_server_caps() {
    let server = MockServer::start().await;
    mount_logs_mint(&server, 1).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let events = rows_of(1_000, |i| format!("{i:04} {}", "x".repeat(600)));
    let (out, _) = run(&server, &events).await;
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 1_000 });
    let got = received(&server).await;
    assert!(got.len() >= 2, "~800 KiB of NDJSON cannot be one chunk");
    let mut total = 0;
    for (gz, rows) in &got {
        let raw: usize = rows.iter().map(|r| r.to_string().len() + 1).sum();
        assert!(raw <= 384 * 1024, "{raw} uncompressed bytes");
        assert!(*gz <= 64 * 1024, "{gz} gzip bytes");
        total += rows.len();
    }
    assert_eq!(total, 1_000);
}

#[tokio::test]
async fn an_incompressible_batch_is_halved_until_every_gzip_body_fits() {
    let server = MockServer::start().await;
    mount_logs_mint(&server, 1).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let events = rows_of(1_000, |i| random_hex(i as u64 + 1, 300));
    // The precondition that makes this test mean something: the first
    // size-based batch alone is over the compressed cap.
    let first = chunk::split_batches(&events, MAX_BATCH_BYTES, MAX_CHUNK_ROWS).unwrap()[0].clone();
    assert!(
        chunk::encode_ndjson_gzip(&events[first]).unwrap().len() > MAX_GZIP_BYTES,
        "the fixture must not gzip under 64 KiB un-halved"
    );
    let (out, chunks) = run(&server, &events).await;
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 1_000 });
    assert_eq!(chunks.iter().sum::<usize>(), 1_000);
    let got = received(&server).await;
    let by_size = chunk::split_batches(&events, MAX_BATCH_BYTES, MAX_CHUNK_ROWS)
        .unwrap()
        .len();
    assert!(
        got.len() > by_size,
        "halving makes more chunks than the size split ({} vs {by_size})",
        got.len()
    );
    assert!(got.iter().all(|(gz, _)| *gz <= MAX_GZIP_BYTES));
    assert_eq!(got.iter().map(|(_, rows)| rows.len()).sum::<usize>(), 1_000);
}

// ---- a row too big on its own ----

/// `rows()` cuts messages at 2,048 bytes, so a row that gzips over 64 KiB has
/// to be built by hand; the planner works on any event.
fn huge_row() -> TelemetryEvent {
    let mut fields = serde_json::Map::new();
    fields.insert("message".into(), random_hex(99, 200_000).into());
    TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms: 1,
        seq: 1,
        target: LAUNCHER_LOG_TARGET.into(),
        level: "info".into(),
        fields,
    })
}

#[test]
fn a_single_row_that_cannot_fit_is_dropped_and_counted_and_the_rest_go() {
    let mut events = rows_of(2, |i| format!("line {i}"));
    events.insert(1, huge_row());
    let steps = plan(&events).unwrap();
    assert_eq!(steps, vec![Step::Send(0..1), Step::Drop, Step::Send(2..3)]);

    let mut pacer = FakePacer::default();
    let mut sink = FakeSink::new(&pacer, 0);
    let mut marks = Vec::new();
    // A second live dispatcher keeps the callsite interest from caching "never".
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let (out, logged) = capture(|| {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(send_uncancelled(&mut sink, &events, &mut pacer, |n| {
                marks.push(n)
            }))
    });
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 2 });
    // The skipped row rides with the chunk after it, so an offset moves past it.
    assert_eq!(marks, vec![1, 2]);
    let finished = logged
        .iter()
        .find(|e| e.field("event") == Some("log_upload_finished"))
        .expect("a finish event");
    assert_eq!(finished.field("rows_dropped"), Some("1"));
}

// ---- the two INFO events ----

#[test]
fn start_and_finish_are_one_info_event_each_without_the_upload_id() {
    let events = rows_of(2_000, |i| format!("line {i}"));
    let mut pacer = FakePacer::default();
    let mut sink = FakeSink::new(&pacer, 0);
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let (out, logged) = capture(|| {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(send_uncancelled(&mut sink, &events, &mut pacer, |_| {}))
    });
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 2_000 });
    let ours: Vec<_> = logged
        .iter()
        .filter(|e| e.target == "launcher.telemetry")
        .collect();
    assert_eq!(ours.len(), 2, "{ours:?}");
    assert_eq!(ours[0].field("event"), Some("log_upload_started"));
    assert_eq!(ours[0].field("lines"), Some("2000"));
    assert_eq!(ours[1].field("event"), Some("log_upload_finished"));
    assert_eq!(ours[1].field("outcome"), Some("sent"));
    assert_eq!(
        (ours[1].field("lines_sent"), ours[1].field("chunks")),
        (Some("2000"), Some("2"))
    );
    for e in &ours {
        assert_eq!(e.level, tracing::Level::INFO);
        assert!(e.fields.values().all(|v| !v.contains(UPLOAD_ID)));
        assert!(
            e.field("count").is_none(),
            "the server denies the key `count`"
        );
    }
}

#[test]
fn a_bad_upload_id_or_target_yields_no_rows() {
    let l = line(1, "x".into());
    let one = std::slice::from_ref(&l);
    assert_eq!(
        rows(one, LAUNCHER_LOG_TARGET, UPLOAD_ID, "v", "win").len(),
        1
    );
    for id in ["0123456789ABCDEF", "0123456789abcde", "0123456789abcdeg"] {
        assert!(
            rows(one, LAUNCHER_LOG_TARGET, id, "v", "win").is_empty(),
            "{id}"
        );
    }
    assert!(rows(&[l], "client.launcher.other", UPLOAD_ID, "v", "win").is_empty());
}

#[test]
fn the_planned_posts_are_capped_at_400() {
    let steps = vec![Step::Send(0..1); MAX_UPLOAD_CHUNKS + 1];
    assert_eq!(
        cap_sends(steps),
        (vec![Step::Send(0..1); MAX_UPLOAD_CHUNKS], true)
    );
}

#[tokio::test]
async fn an_upload_over_40000_rows_sends_40000_and_ends_partial() {
    let events = rows_of(40_001, |i| format!("line {i}"));
    let mut pacer = FakePacer::default();
    let mut sink = FakeSink::new(&pacer, 0);
    let mut marks = Vec::new();
    let out = send_uncancelled(&mut sink, &events, &mut pacer, |n| marks.push(n)).await;
    assert_eq!(
        out,
        IngestOutcome::Partial {
            lines_sent: 40_000,
            retry_after_secs: 60
        }
    );
    assert_eq!(marks.iter().sum::<usize>(), 40_000);
}
