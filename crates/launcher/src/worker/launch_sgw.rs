//! Launching `SGW.exe`: the always-injected client-patches DLL
//! (Black Market plan D2, BM-06), the plain-launch fallback, and the
//! telemetry session that follows the game when the player opted in.
//!
//! The launcher is 64-bit and `SGW.exe` is 32-bit, so DLLs go in through
//! the 32-bit `sgw-start32` helper (`cimmeria_client_launch::start32`): it
//! starts the game suspended, injects, resumes, and hands back the pid,
//! which the launcher then follows.
//!
//! Every launch tells the player what happened to the patches in the
//! status log, including when they were skipped, so a missing Black
//! Market window is never a silent mystery.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use cimmeria_client_launch::inject::RunningProcess;
use cimmeria_client_launch::start32::{self, Request, Target};
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::{launch_telemetry, Event, LaunchSgwRequest, Worker};
use crate::client_patches::{decide, dll_source, injection_order, InjectDecision, PatchInjection};
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
    /// can start it plainly.
    Failed(String),
}

impl Worker {
    pub(super) fn spawn_launch_sgw(&self, req: LaunchSgwRequest) {
        let events_tx = self.events_tx.clone();
        // Only telemetry uses it.
        let http = self.telemetry_http.clone();
        self.runtime.spawn(async move {
            let launched_at = SystemTime::now();
            let sgw_dir = sgw_dir(&req.install_dir);
            // A missing game fails once, before any telemetry handshake.
            if let Err(e) = checked_sgw_exe(&sgw_dir) {
                let _ = events_tx.send(Event::LaunchError(e.to_string()));
                return;
            }
            // Opted in: the session file must exist before the game starts,
            // because the in-game DLL reads it once, at start.
            let session = match &req.telemetry {
                Some(cfg) => {
                    launch_telemetry::open_session(&http, &req.install_dir, cfg, &events_tx)
                        .await
                        .map(|t| (t, cfg.clone()))
                }
                None => None,
            };
            let telemetry_dll = session
                .as_ref()
                .and_then(|(_, cfg)| launch_telemetry::telemetry_dll(cfg, &events_tx));
            let Some(game) = start_game(
                &sgw_dir,
                &req.client_patches,
                telemetry_dll.as_deref(),
                &events_tx,
            ) else {
                return;
            };
            let Some((telemetry, cfg)) = session else {
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
            launch_telemetry::follow(
                http,
                telemetry,
                req.install_dir,
                cfg,
                exit,
                patch_log,
                &events_tx,
            )
            .await;
        });
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
    let request = Request {
        target: Target::Spawn {
            exe,
            cwd: Some(dir),
            args: Vec::new(),
        },
        dlls: dlls.to_vec(),
    };
    match start32::run(&helper, &request) {
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

/// Start `SGW.exe`, with the client-patches DLL unless the player opted
/// out or it is unavailable. A failed injection falls back to a plain
/// launch: the patches are never worth a game that will not start.
fn start_game(
    install_dir: &Path,
    settings: &ClientPatchesSettings,
    telemetry_dll: Option<&Path>,
    events_tx: &mpsc::UnboundedSender<Event>,
) -> Option<StartedGame> {
    let decision = decide(settings, || dll_source::resolve(settings, &exe_dir()));
    report_skipped(&decision, events_tx);
    let mut injection = match &decision {
        InjectDecision::Inject(_) => PatchInjection::Injected,
        InjectDecision::OptedOut => PatchInjection::OptedOut,
        InjectDecision::Unavailable(_) => PatchInjection::Unavailable,
    };

    let patches = match &decision {
        InjectDecision::Inject(src) => Some(src.path().to_path_buf()),
        _ => None,
    };
    let dlls = injection_order(patches.as_deref(), telemetry_dll);
    if !dlls.is_empty() {
        match start_via_helper(install_dir, &dlls) {
            HelperStart::Started { pid, exit } => {
                info!(pid, dlls = ?dlls, "SGW.exe launched through the helper");
                let what = if patches.is_some() {
                    "SGW.exe with client patches"
                } else {
                    "SGW.exe"
                };
                let _ = events_tx.send(Event::Launched(what.into(), pid));
                if let Some(dll) = telemetry_dll {
                    launch_telemetry::report_loaded(dll, events_tx);
                }
                return Some(StartedGame { exit, injection });
            }
            HelperStart::GameMissing(e) => {
                let _ = events_tx.send(Event::LaunchError(e.to_string()));
                return None;
            }
            HelperStart::Failed(why) => {
                warn!(reason = %why, dlls = ?dlls, "DLL injection failed; launching without the DLLs");
                if patches.is_some() {
                    let _ = events_tx.send(Event::ClientPatchesNote(format!(
                        "not loaded ({why}); starting the game without them. \
                         The Black Market window will not open."
                    )));
                    injection = PatchInjection::InjectFailed;
                }
                if telemetry_dll.is_some() {
                    let _ = events_tx.send(Event::TelemetryNote(format!(
                        "not loaded ({why}). Log upload still runs."
                    )));
                }
            }
        }
    }

    match launch_sgw_with_child(install_dir) {
        Ok(child) => {
            let _ = events_tx.send(Event::Launched("SGW.exe".into(), child.id()));
            Some(StartedGame {
                exit: Some(Box::pin(wait_for_exit(child))),
                injection,
            })
        }
        Err(e) => {
            let _ = events_tx.send(Event::LaunchError(e.to_string()));
            None
        }
    }
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

#[cfg(test)]
mod tests {
    use super::super::tests::{make_worker, recv_matching};
    use super::super::Command;
    use super::*;

    fn opted_out() -> ClientPatchesSettings {
        ClientPatchesSettings {
            enabled: false,
            dll_override: None,
        }
    }

    /// With no SGW.exe the launch fails once, with the game's error,
    /// before any DLL decision or telemetry handshake.
    #[test]
    fn launch_sgw_missing_game_fails_first_and_once() {
        let dir = tempfile::tempdir().unwrap();
        let (mut worker, rt) = make_worker();
        worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
            install_dir: dir.path().to_path_buf(),
            client_patches: opted_out(),
            telemetry: None,
        }));
        let first = rt.block_on(recv_matching(&mut worker.events_rx, |_| true));
        match first {
            Event::LaunchError(msg) => assert!(msg.contains("SGW.exe"), "{msg}"),
            other => panic!("expected LaunchError first, got {other:?}"),
        }
    }

    /// A configured DLL that is missing is reported, and the launch
    /// carries on without it.
    #[test]
    fn launch_sgw_reports_a_missing_override_dll() {
        let dir = tempfile::tempdir().unwrap();
        // A game must exist to get as far as the DLL decision.
        std::fs::write(dir.path().join("SGW.exe"), b"not a real game").unwrap();
        let (mut worker, rt) = make_worker();
        worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
            install_dir: dir.path().to_path_buf(),
            client_patches: ClientPatchesSettings {
                enabled: true,
                dll_override: Some(dir.path().join("missing.dll")),
            },
            telemetry: None,
        }));
        let note = rt.block_on(recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::ClientPatchesNote(_))
        }));
        match note {
            Event::ClientPatchesNote(n) => assert!(n.contains("missing.dll"), "{n}"),
            other => panic!("expected ClientPatchesNote, got {other:?}"),
        }
    }

    /// With an injectable DLL but no SGW.exe, the error is the game's,
    /// reported once, and the patches are not blamed.
    #[test]
    fn launch_sgw_missing_game_is_not_blamed_on_the_patches() {
        let dir = tempfile::tempdir().unwrap();
        let dll = dir.path().join("p.dll");
        std::fs::write(&dll, b"MZ").unwrap();
        let (mut worker, rt) = make_worker();
        worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
            install_dir: dir.path().to_path_buf(),
            client_patches: ClientPatchesSettings {
                enabled: true,
                dll_override: Some(dll),
            },
            telemetry: None,
        }));
        let ev = rt.block_on(recv_matching(&mut worker.events_rx, |e| {
            matches!(
                e,
                Event::Launched(..) | Event::LaunchError(_) | Event::ClientPatchesNote(_)
            )
        }));
        match ev {
            Event::LaunchError(msg) => assert!(msg.contains("SGW.exe"), "{msg}"),
            other => panic!("expected only a LaunchError, got {other:?}"),
        }
    }

    /// A build with the patches DLL but no `sgw-start32.exe` (a dev build:
    /// nothing is embedded in tests and none sits beside the test binary)
    /// says so, then still starts the game plainly.
    #[cfg(windows)]
    #[test]
    fn launch_sgw_without_the_helper_reports_it_and_launches_plainly() {
        let sys = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(sys.join("HOSTNAME.EXE"), dir.path().join("SGW.exe")).unwrap();
        let dll = dir.path().join("p.dll");
        std::fs::write(&dll, b"MZ").unwrap();
        let (mut worker, rt) = make_worker();
        worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
            install_dir: dir.path().to_path_buf(),
            client_patches: ClientPatchesSettings {
                enabled: true,
                dll_override: Some(dll),
            },
            telemetry: None,
        }));
        let (note, launched) = rt.block_on(async {
            let note = recv_matching(&mut worker.events_rx, |e| {
                matches!(e, Event::ClientPatchesNote(_))
            })
            .await;
            let launched = recv_matching(&mut worker.events_rx, |e| {
                matches!(e, Event::Launched(..) | Event::LaunchError(_))
            })
            .await;
            (note, launched)
        });
        match note {
            Event::ClientPatchesNote(n) => {
                assert!(n.contains("sgw-start32.exe"), "{n}");
                assert!(n.contains("not loaded"), "{n}");
            }
            other => panic!("expected ClientPatchesNote, got {other:?}"),
        }
        match launched {
            Event::Launched(name, _) => assert_eq!(name, "SGW.exe"),
            other => panic!("expected a plain launch, got {other:?}"),
        }
    }
}
