//! Launching `SGW.exe`: the always-injected client-patches DLL
//! (Black Market plan D2, BM-06), the telemetry DLL when the player opted
//! in to telemetry, and the fallbacks that keep the game starting when
//! either cannot go in.
//!
//! The launcher is 64-bit and `SGW.exe` is 32-bit, so DLLs go in through
//! the 32-bit `sgw-start32` helper (`cimmeria_client_launch::start32`): it
//! starts the game suspended, injects, resumes, and hands back the pid,
//! which the launcher then follows.
//!
//! An opted-in launch starts the telemetry session *before* the game
//! ([`super::launch_telemetry`]): the telemetry DLL reads its upload token
//! from `current-session.json` as it boots, so the file must be there
//! first. Without the opt-in, nothing here changes from a launch without
//! telemetry.
//!
//! Every launch tells the player what happened to each DLL in the status
//! log, including when it was skipped, so a missing Black Market window
//! or a silent telemetry session is never a mystery.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use cimmeria_client_launch::inject::RunningProcess;
use cimmeria_client_launch::start32::{self, Request, Target};
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::launch_telemetry::{follow_with_telemetry, start_player_session};
use super::{Event, LaunchSgwRequest, Worker};
use crate::client_patches::{decide, dll_source, injection_order, InjectDecision, PatchInjection};
use crate::client_telemetry_dll::{self, DllOutcome, TelemetryDll};
use crate::config::{exe_dir, ClientPatchesSettings};
use crate::install_layout;
use crate::launch::{checked_sgw_exe, launch_sgw_with_child, LaunchError};
use crate::start32_helper;
use crate::telemetry::patch_log::PatchLogWatcher;
use crate::telemetry::process_watch::{wait_for_exit, wait_for_running_exit, ExitWaiter};

/// The directory holding `SGW.exe`, which is also its working directory.
///
/// A real install keeps `SGW.exe` in `<install>\Working\Binaries`
/// ([`install_layout::binaries_dir`]). Every `SGW.exe` launch in this module
/// (the helper's exe and cwd, the plain-launch fallback, and the patch log
/// next to `SGW.exe`) goes through this one function.
fn sgw_dir(install_dir: &Path) -> PathBuf {
    install_layout::binaries_dir(install_dir)
}

/// A started game. `exit` is `None` when the game started but its pid
/// could not be opened to wait on: it runs, but no telemetry session can
/// follow it.
struct StartedGame {
    exit: Option<ExitWaiter>,
    injection: PatchInjection,
    /// `None` when the telemetry DLL was never wanted (no opt-in, or no
    /// session to give it a token).
    telemetry_dll: Option<DllOutcome>,
}

/// How a launch through the helper went.
enum HelperStart {
    Started {
        pid: u32,
        exit: Option<ExitWaiter>,
    },
    /// `SGW.exe` itself is missing or escapes the install dir: a plain
    /// launch would fail the same way, so the caller reports it once.
    GameMissing(LaunchError),
    /// The DLLs could not go in (no helper, or the helper failed). The
    /// helper kills a suspended game whose injection failed, so the caller
    /// can start it again.
    Failed(String),
}

impl Worker {
    pub(super) fn spawn_launch_sgw(&self, req: LaunchSgwRequest) {
        let events_tx = self.events_tx.clone();
        // Only the telemetry session uses it.
        let http = self.telemetry_http.clone();
        self.runtime.spawn(async move {
            let launched_at = SystemTime::now();
            let sgw_dir = sgw_dir(&req.install_dir);
            // Opted in: session first, then the DLL that reads it.
            let session = match req.telemetry {
                Some(cfg) => {
                    Some(start_player_session(&http, &req.install_dir, cfg, &events_tx).await)
                }
                None => None,
            };
            let telemetry_dll = match &session {
                None => TelemetryDll::NotOptedIn,
                Some(s) => {
                    TelemetryDll::decide(s.started(), || client_telemetry_dll::resolve(&exe_dir()))
                }
            };
            let Some(game) = start_game(&sgw_dir, &req.client_patches, &telemetry_dll, &events_tx)
            else {
                return;
            };
            let Some(session) = session else {
                return;
            };
            let Some(exit) = game.exit else {
                let _ = events_tx.send(Event::TelemetrySessionError(
                    "the game started, but its process could not be opened to follow it; \
                     no telemetry session this launch"
                        .into(),
                ));
                return;
            };
            let patch_log = PatchLogWatcher::new(&sgw_dir, launched_at, game.injection);
            follow_with_telemetry(
                http,
                req.install_dir,
                session,
                &telemetry_dll,
                game.telemetry_dll,
                exit,
                patch_log,
                &events_tx,
            )
            .await;
        });
    }
}

/// The DLL sets to try, in order, until one starts the game; if all fail
/// the game starts plainly. With both DLLs, the second try drops the
/// telemetry DLL, so a telemetry failure never costs the player the
/// client patches.
fn launch_attempts(patches: Option<&Path>, telemetry: Option<&Path>) -> Vec<Vec<PathBuf>> {
    let mut attempts = Vec::new();
    let first = injection_order(patches, telemetry);
    if !first.is_empty() {
        attempts.push(first);
    }
    if let (Some(p), Some(_)) = (patches, telemetry) {
        attempts.push(injection_order(Some(p), None));
    }
    attempts
}

/// The helper request for one attempt: start `exe` suspended in `dir`,
/// inject `dlls` in order, resume. Pure so the command line is testable.
fn helper_request(dir: PathBuf, exe: PathBuf, dlls: &[PathBuf]) -> Request {
    Request {
        target: Target::Spawn {
            exe,
            cwd: Some(dir),
            args: Vec::new(),
        },
        dlls: dlls.to_vec(),
    }
}

/// Start `SGW.exe` through the helper with `dlls` injected in order, and
/// open the pid it reports to follow the game. `sgw_dir` is the directory
/// holding `SGW.exe` ([`sgw_dir`]); it is also the game's working directory.
fn start_via_helper(sgw_dir: &Path, dlls: &[PathBuf]) -> HelperStart {
    let (dir, exe) = match checked_sgw_exe(sgw_dir) {
        Ok(paths) => paths,
        Err(e) => return HelperStart::GameMissing(e),
    };
    let helper = match start32_helper::resolve(&exe_dir()) {
        Ok(h) => h,
        Err(e) => return HelperStart::Failed(e.to_string()),
    };
    match start32::run(&helper, &helper_request(dir, exe, dlls)) {
        Ok(pid) => {
            let exit = match RunningProcess::open(pid) {
                Ok(process) => Some(Box::pin(wait_for_running_exit(process)) as ExitWaiter),
                Err(e) => {
                    warn!(pid, error = %e, "started SGW.exe but could not open it to follow");
                    None
                }
            };
            HelperStart::Started { pid, exit }
        }
        Err(e) => HelperStart::Failed(e.to_string()),
    }
}

/// Start `SGW.exe` with the client-patches DLL unless the player opted
/// out or it is unavailable, and the telemetry DLL when `telemetry` says
/// to. A failed injection falls back to fewer DLLs, then to a plain
/// launch: neither DLL is worth a game that will not start.
fn start_game(
    sgw_dir: &Path,
    settings: &ClientPatchesSettings,
    telemetry: &TelemetryDll,
    events_tx: &mpsc::UnboundedSender<Event>,
) -> Option<StartedGame> {
    let decision = decide(settings, || dll_source::resolve(settings, &exe_dir()));
    report_skipped(&decision, events_tx);
    report_telemetry_skipped(telemetry, events_tx);
    let patches = match &decision {
        InjectDecision::Inject(src) => Some(src.path().to_path_buf()),
        _ => None,
    };
    let tel = telemetry.path();
    let mut injection = match &decision {
        InjectDecision::Inject(_) => PatchInjection::Injected,
        InjectDecision::OptedOut => PatchInjection::OptedOut,
        InjectDecision::Unavailable(_) => PatchInjection::Unavailable,
    };
    let mut tel_outcome = match telemetry {
        TelemetryDll::Inject(_) => Some(DllOutcome::Injected),
        TelemetryDll::Unavailable(_) => Some(DllOutcome::Unavailable),
        TelemetryDll::NotOptedIn | TelemetryDll::NoSession(_) => None,
    };

    for dlls in launch_attempts(patches.as_deref(), tel) {
        let has_patches = patches.as_ref().is_some_and(|p| dlls.contains(p));
        let has_tel = tel.is_some_and(|t| dlls.iter().any(|d| d == t));
        match start_via_helper(sgw_dir, &dlls) {
            HelperStart::Started { pid, exit } => {
                info!(pid, dlls = ?dlls, "SGW.exe launched with DLLs");
                let _ = events_tx.send(Event::Launched(launched_label(has_patches, has_tel), pid));
                return Some(StartedGame {
                    exit,
                    injection,
                    telemetry_dll: tel_outcome,
                });
            }
            HelperStart::GameMissing(e) => {
                let _ = events_tx.send(Event::LaunchError(e.to_string()));
                return None;
            }
            HelperStart::Failed(why) => {
                warn!(reason = %why, dlls = ?dlls, "DLL injection failed; retrying with fewer DLLs");
                if has_tel {
                    tel_outcome = Some(DllOutcome::InjectFailed);
                    let _ = events_tx.send(Event::ClientTelemetryNote(format!(
                        "not loaded ({why}); starting the game without in-game telemetry."
                    )));
                } else if has_patches {
                    injection = PatchInjection::InjectFailed;
                    let _ = events_tx.send(Event::ClientPatchesNote(format!(
                        "not loaded ({why}); starting the game without them. \
                         The Black Market window will not open."
                    )));
                }
            }
        }
    }
    // Here every attempt failed. The last one carried the patches alone
    // whenever there were patches, so `injection` already says so.
    match launch_sgw_with_child(sgw_dir) {
        Ok(child) => {
            let _ = events_tx.send(Event::Launched("SGW.exe".into(), child.id()));
            Some(StartedGame {
                exit: Some(Box::pin(wait_for_exit(child))),
                injection,
                telemetry_dll: tel_outcome,
            })
        }
        Err(e) => {
            let _ = events_tx.send(Event::LaunchError(e.to_string()));
            None
        }
    }
}

fn launched_label(patches: bool, telemetry: bool) -> String {
    match (patches, telemetry) {
        (true, true) => "SGW.exe with client patches and telemetry",
        (true, false) => "SGW.exe with client patches",
        (false, true) => "SGW.exe with telemetry",
        (false, false) => "SGW.exe",
    }
    .into()
}

/// Tell the player, and the launcher log, why the patches are not going in.
fn report_skipped(decision: &InjectDecision, events_tx: &mpsc::UnboundedSender<Event>) {
    let note = match decision {
        InjectDecision::Inject(_) => return,
        InjectDecision::OptedOut => {
            info!("client patches off by launcher setting");
            "off (launcher setting). The Black Market window will not open.".to_string()
        }
        InjectDecision::Unavailable(why) => {
            warn!(reason = %why, "client patches unavailable");
            format!("unavailable: {why}. The Black Market window will not open.")
        }
    };
    let _ = events_tx.send(Event::ClientPatchesNote(note));
}

/// Tell the player why an opted-in launch goes without the telemetry DLL.
/// A failed session start was already reported as a session error.
fn report_telemetry_skipped(telemetry: &TelemetryDll, events_tx: &mpsc::UnboundedSender<Event>) {
    if let TelemetryDll::Unavailable(why) = telemetry {
        warn!(reason = %why, "client telemetry DLL unavailable; launching without it");
        let _ = events_tx.send(Event::ClientTelemetryNote(format!(
            "unavailable: {why}. The game starts without in-game telemetry; \
             the launcher still uploads the client's logs."
        )));
    }
}

#[cfg(test)]
#[path = "launch_sgw_tests.rs"]
mod tests;
