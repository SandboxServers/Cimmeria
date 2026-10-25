use super::*;
use crate::test_capture::capture;

const UPLOAD_ID: &str = "0123456789abcdef";

/// Answers every post as accepted and reports a fixed endpoint, as a sender would.
struct EndpointSink(Option<(String, String)>);

impl ChunkSink for EndpointSink {
    fn post(&mut self, _batch: &[TelemetryEvent]) -> impl Future<Output = BatchOutcome> + Send {
        std::future::ready(BatchOutcome::Posted)
    }

    fn endpoint(&self) -> Option<(String, String)> {
        self.0.clone()
    }
}

/// Refuses every post, and reports a fixed endpoint, as a sender would.
struct FailingSink(Option<(String, String)>);

impl ChunkSink for FailingSink {
    fn post(&mut self, _batch: &[TelemetryEvent]) -> impl Future<Output = BatchOutcome> + Send {
        std::future::ready(BatchOutcome::Failed)
    }

    fn endpoint(&self) -> Option<(String, String)> {
        self.0.clone()
    }
}

/// One upload of a single small row through `sink`; returns the outcome and the captured events.
fn upload_one<S: ChunkSink + Send>(
    sink: &mut S,
) -> (IngestOutcome, Vec<crate::test_capture::Captured>) {
    let lines = [LogLine {
        file: "launcher.log".into(),
        line_no: 1,
        end: 100,
        ts_ms: None,
        level: None,
        target: None,
        message: "line".into(),
        truncated: false,
    }];
    let events = rows(&lines, LAUNCHER_LOG_TARGET, UPLOAD_ID, "v", "win");
    let mut pacer = TokioPacer::new();
    // A second live dispatcher keeps the callsite interest from caching "never".
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    capture(|| {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(send_with(
                sink,
                &events,
                &mut pacer,
                &CancellationToken::new(),
                |_| {},
            ))
    })
}

#[test]
fn the_finish_line_names_the_scheme_and_host_of_the_last_post() {
    let mut sink = EndpointSink(Some(("https".into(), "play.example".into())));
    let (out, logged) = upload_one(&mut sink);
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 1 });
    let finished = logged
        .iter()
        .find(|e| e.field("event") == Some("log_upload_finished"))
        .expect("a finish event");
    assert_eq!(finished.field("scheme"), Some("https"));
    assert_eq!(finished.field("host"), Some("play.example"));
}

#[test]
fn the_finish_line_says_dash_when_no_endpoint_is_known() {
    let mut sink = EndpointSink(None);
    let (out, logged) = upload_one(&mut sink);
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 1 });
    let finished = logged
        .iter()
        .find(|e| e.field("event") == Some("log_upload_finished"))
        .expect("a finish event");
    assert_eq!(finished.field("scheme"), Some("-"));
    assert_eq!(finished.field("host"), Some("-"));
}

#[test]
fn a_failed_finish_line_names_the_scheme_and_host_too() {
    let mut sink = FailingSink(Some(("https".into(), "play.example".into())));
    let (out, logged) = upload_one(&mut sink);
    assert_eq!(out, IngestOutcome::Failed);
    let finished = logged
        .iter()
        .find(|e| {
            e.field("event") == Some("log_upload_finished") && e.field("outcome") == Some("failed")
        })
        .expect("a failed finish event");
    assert_eq!(finished.field("scheme"), Some("https"));
    assert_eq!(finished.field("host"), Some("play.example"));
}
