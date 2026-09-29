//! The telemetry half of "Launch SGW.exe", for players who opted in.
//!
//! The in-game DLL (`cimmeria-client-telemetry.dll`) reads
//! `current-session.json` once, when it starts, and stays idle without it.
//! So the order is: handshake first ([`open_session`], bounded by
//! [`HANDSHAKE_TIMEOUT`] so telemetry never holds the game up for long),
//! which writes that file; then the game starts with the DLL
//! ([`telemetry_dll`]); then the session follows the game ([`follow`]).
//! If the handshake fails the game starts anyway, without the DLL.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::{Event, LaunchTelemetryConfig};
use crate::config::exe_dir;
use crate::telemetry::auth::DevSessionRequest;
use crate::telemetry::dll_source;
use crate::telemetry::patch_log::PatchLogWatcher;
use crate::telemetry::process_watch::ExitWaiter;
use crate::telemetry::runner::run_session;
use crate::telemetry::Telemetry;

/// Longest the telemetry handshake may delay the game's start.
pub(super) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Mint the session and write `current-session.json`. `None` (with the
/// reason in the status log) when the handshake fails or times out.
pub(super) async fn open_session(
    http: &reqwest::Client,
    install_dir: &Path,
    cfg: &LaunchTelemetryConfig,
    events_tx: &mpsc::UnboundedSender<Event>,
) -> Option<Telemetry> {
    let req = DevSessionRequest {
        install_id: cfg.install_id.clone(),
        machine_id: cfg.machine_id.clone(),
        branch: cfg.branch.clone(),
        git_sha: cfg.git_sha.clone(),
        launcher_version: cfg.launcher_version.clone(),
        tags: cfg.tags.clone(),
    };
    let start = Telemetry::start_session(
        http,
        &cfg.auth_base_url,
        req,
        install_dir,
        &cfg.launcher_version,
    );
    let why = match tokio::time::timeout(HANDSHAKE_TIMEOUT, start).await {
        Ok(Ok(t)) => return Some(t),
        Ok(Err(e)) => e.to_string(),
        Err(_) => format!(
            "the telemetry server did not answer within {} s",
            HANDSHAKE_TIMEOUT.as_secs()
        ),
    };
    error!(reason = %why, "telemetry session start failed");
    let _ = events_tx.send(Event::TelemetrySessionError(format!(
        "auth handshake failed: {why}. The game starts without telemetry."
    )));
    None
}

/// The in-game DLL for a launch whose session is open, or `None` with the
/// reason in the status log. The launcher-side session (log tailing and
/// uploads) runs either way.
pub(super) fn telemetry_dll(
    cfg: &LaunchTelemetryConfig,
    events_tx: &mpsc::UnboundedSender<Event>,
) -> Option<PathBuf> {
    match dll_source::resolve(cfg.dll_override.as_deref(), &exe_dir()) {
        Ok(path) => Some(path),
        Err(e) => {
            warn!(reason = %e, "in-game telemetry DLL unavailable");
            let _ = events_tx.send(Event::TelemetryNote(format!(
                "not loaded: {e}. Log upload still runs."
            )));
            None
        }
    }
}

/// Report that the DLL went in, once the game has started with it.
pub(super) fn report_loaded(dll: &Path, events_tx: &mpsc::UnboundedSender<Event>) {
    info!(dll = %dll.display(), "in-game telemetry DLL injected");
    let _ = events_tx.send(Event::TelemetryNote(format!(
        "loaded ({}). Its own log is cimmeria-client-telemetry.log beside SGW.exe.",
        dll.display()
    )));
}

/// Run the open session until the game exits.
pub(super) async fn follow(
    http: reqwest::Client,
    telemetry: Telemetry,
    install_dir: PathBuf,
    cfg: LaunchTelemetryConfig,
    exit: ExitWaiter,
    patch_log: PatchLogWatcher,
    events_tx: &mpsc::UnboundedSender<Event>,
) {
    match run_session(
        Arc::new(telemetry),
        Arc::new(http),
        exit,
        install_dir,
        cfg.state_dir,
        Some(patch_log),
    )
    .await
    {
        Ok(outcome) => {
            let _ = events_tx.send(Event::TelemetrySessionComplete(outcome));
        }
        Err(e) => {
            error!("telemetry session error: {e}");
            let _ = events_tx.send(Event::TelemetrySessionError(e.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::endpoint;
    use crate::telemetry::session::current_session_path;

    fn cfg(auth: &str) -> LaunchTelemetryConfig {
        LaunchTelemetryConfig {
            auth_base_url: auth.into(),
            install_id: "i".into(),
            machine_id: "m".into(),
            branch: "b".into(),
            git_sha: "g".into(),
            launcher_version: "0.1.0".into(),
            state_dir: PathBuf::from("."),
            tags: vec![],
            dll_override: None,
        }
    }

    /// The DLL reads `current-session.json` only when it starts, so the
    /// file must exist before the game does: `open_session` writes it.
    #[tokio::test]
    async fn open_session_writes_the_session_file_the_dll_reads() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth/dev-session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sid-1",
                "token": "tok",
                "expires_at_ms": 1_700_028_800_000_i64,
                "upload_endpoint": format!("{}/api/telemetry", server.uri()),
                "chunk_max_bytes": 1_048_576,
                "flush_interval_ms": 2_000,
            })))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        let t = open_session(&endpoint::client(), dir.path(), &cfg(&server.uri()), &tx).await;
        assert!(t.is_some());
        assert!(current_session_path(dir.path()).is_file());
    }

    /// A failed handshake is reported and yields no session, so the
    /// launch goes ahead without the DLL.
    #[tokio::test]
    async fn a_failed_handshake_reports_and_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let t = open_session(
            &endpoint::client(),
            dir.path(),
            &cfg("http://telemetry.invalid/api"),
            &tx,
        )
        .await;
        assert!(t.is_none());
        match rx.try_recv().unwrap() {
            Event::TelemetrySessionError(m) => assert!(m.contains("without telemetry"), "{m}"),
            other => panic!("expected TelemetrySessionError, got {other:?}"),
        }
        assert!(!current_session_path(dir.path()).exists());
    }

    #[test]
    fn a_missing_override_dll_is_reported_not_loaded() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut c = cfg("http://localhost/api");
        c.dll_override = Some(PathBuf::from("Z:/nope/telemetry.dll"));
        assert_eq!(telemetry_dll(&c, &tx), None);
        match rx.try_recv().unwrap() {
            Event::TelemetryNote(m) => assert!(m.contains("not loaded"), "{m}"),
            other => panic!("expected TelemetryNote, got {other:?}"),
        }
    }
}
