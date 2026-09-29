//! The telemetry session of an opted-in `SGW.exe` launch: started before
//! the game, so `current-session.json` is on disk when the telemetry DLL
//! boots, then run for as long as the game does.
//!
//! The launcher's dev-session request carries no `session_kind`, so the
//! server mints a `player` token; the DLL uploads with that same token,
//! and its rows carry `cimmeria.session_kind = player`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tracing::{error, info, warn};

use super::{Event, EventSender, LaunchTelemetryConfig};
use crate::client_telemetry_dll::{DllOutcome, TelemetryDll};
use crate::telemetry::auth::DevSessionRequest;
use crate::telemetry::endpoint::EndpointPolicy;
use crate::telemetry::events::{ClientNativeEvent, TelemetryEvent};
use crate::telemetry::patch_log::PatchLogWatcher;
use crate::telemetry::process_watch::ExitWaiter;
use crate::telemetry::runner::run_session;
use crate::telemetry::Telemetry;

/// How long the game launch waits for the auth handshake. The session
/// now comes before the game, so an unreachable telemetry server must
/// not hold up play for longer than this.
const SESSION_START_TIMEOUT: Duration = Duration::from_secs(10);

/// Target of the launcher's once-per-launch event about the DLL.
pub(super) const DLL_EVENT_TARGET: &str = "client.telemetry_dll.launch";

/// An opted-in launch's session: started, or why not.
pub(super) struct PlayerSession {
    telemetry: Result<Arc<Telemetry>, String>,
    state_dir: PathBuf,
}

impl PlayerSession {
    /// `Ok` when there is a session whose token the DLL can use.
    pub(super) fn started(&self) -> Result<(), String> {
        self.telemetry.as_ref().map(|_| ()).map_err(Clone::clone)
    }
}

/// Mint the session and write `current-session.json`. A failure is
/// reported and the game still launches, without telemetry.
pub(super) async fn start_player_session(
    http: &reqwest::Client,
    install_dir: &std::path::Path,
    cfg: LaunchTelemetryConfig,
    events_tx: &EventSender,
) -> PlayerSession {
    let req = DevSessionRequest {
        install_id: cfg.install_id,
        machine_id: cfg.machine_id,
        branch: cfg.branch,
        git_sha: cfg.git_sha,
        launcher_version: cfg.launcher_version.clone(),
        tags: cfg.tags,
    };
    let started = tokio::time::timeout(
        SESSION_START_TIMEOUT,
        Telemetry::start_session(
            http,
            &cfg.auth_base_url,
            req,
            install_dir,
            &cfg.launcher_version,
            EndpointPolicy::from_login_servers(cfg.login_server_urls.iter().map(String::as_str)),
        ),
    )
    .await;
    let telemetry = match started {
        Ok(Ok(t)) => Ok(Arc::new(t)),
        Ok(Err(e)) => Err(format!("auth handshake failed: {e}")),
        Err(_) => Err(format!(
            "auth handshake timed out after {}s",
            SESSION_START_TIMEOUT.as_secs()
        )),
    };
    if let Err(why) = &telemetry {
        error!(reason = %why, "telemetry session start failed; launching without telemetry");
        let _ = events_tx.send(Event::TelemetrySessionError(why.clone()));
    }
    PlayerSession {
        telemetry,
        state_dir: cfg.state_dir,
    }
}

/// Run the session for an already-started game. The game is running
/// whatever happens here: telemetry never blocks play.
pub(super) async fn follow_with_telemetry(
    http: reqwest::Client,
    install_dir: PathBuf,
    session: PlayerSession,
    dll: &TelemetryDll,
    dll_outcome: Option<DllOutcome>,
    exit: ExitWaiter,
    patch_log: PatchLogWatcher,
    events_tx: &EventSender,
) {
    let Ok(telemetry) = session.telemetry else {
        return;
    };
    if let Some(outcome) = dll_outcome {
        info!(
            outcome = outcome.as_str(),
            "client telemetry DLL launch outcome"
        );
        if let Err(e) = telemetry.enqueue(dll_event(outcome, dll)).await {
            warn!(error = %e, "could not queue the telemetry DLL launch event");
        }
    }
    match run_session(
        telemetry,
        Arc::new(http),
        exit,
        install_dir,
        session.state_dir,
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

/// The `client.telemetry_dll.launch` event: whether the DLL went in, so
/// "no `client.dll.attached` row" can be told apart from "never injected"
/// in SigNoz.
fn dll_event(outcome: DllOutcome, dll: &TelemetryDll) -> TelemetryEvent {
    let mut fields = serde_json::Map::new();
    fields.insert("outcome".into(), outcome.as_str().into());
    match dll {
        TelemetryDll::Inject(path) => {
            fields.insert("dll_path".into(), path.display().to_string().into());
        }
        TelemetryDll::Unavailable(why) => {
            fields.insert("reason".into(), why.clone().into());
        }
        TelemetryDll::NotOptedIn | TelemetryDll::NoSession(_) => {}
    }
    TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms: 0,
        seq: 0,
        target: DLL_EVENT_TARGET.into(),
        level: if outcome == DllOutcome::Injected {
            "info"
        } else {
            "warn"
        }
        .into(),
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(ev: &TelemetryEvent) -> (&str, &str, &serde_json::Map<String, serde_json::Value>) {
        match ev {
            TelemetryEvent::ClientNative(e) => (&e.target, &e.level, &e.fields),
            other => panic!("expected ClientNative, got {other:?}"),
        }
    }

    #[test]
    fn an_injected_dll_is_an_info_event_with_its_path() {
        let ev = dll_event(
            DllOutcome::Injected,
            &TelemetryDll::Inject(PathBuf::from("C:/L/t.dll")),
        );
        let (target, level, f) = fields(&ev);
        assert_eq!(target, "client.telemetry_dll.launch");
        assert_eq!(level, "info");
        assert_eq!(f["outcome"], "injected");
        assert_eq!(f["dll_path"], "C:/L/t.dll");
    }

    #[test]
    fn a_missing_dll_is_a_warn_event_with_the_reason() {
        let ev = dll_event(
            DllOutcome::Unavailable,
            &TelemetryDll::Unavailable("not bundled".into()),
        );
        let (_, level, f) = fields(&ev);
        assert_eq!(level, "warn");
        assert_eq!(f["outcome"], "unavailable");
        assert_eq!(f["reason"], "not bundled");
    }
}
