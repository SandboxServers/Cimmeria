//! Background worker.
//!
//! Hosts the tokio runtime and turns UI [`Command`]s into spawned tasks
//! that emit [`Event`]s back on an unbounded channel. The egui app
//! polls the channel each frame via `events_rx.try_recv()`; every send
//! goes through [`EventSender`], which wakes the UI so that frame happens
//! without mouse input.

mod activity;
mod event_sender;
mod launch_sgw;
mod launch_telemetry;
mod messages;
mod open_folder;
mod self_update;

pub use activity::Busy;
use activity::{Activity, Conflict};
#[cfg(test)]
use event_sender::no_waker;
pub use event_sender::{EventSender, Waker};
pub use messages::{Command, Event, LaunchSgwRequest, LaunchTelemetryConfig};
pub use self_update::UpdateEvent;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::client_paths::{cache_dir, firesky_root, wipe_dir_contents};
use crate::config::LauncherConfig;
use crate::install::{adopt_existing_install, install_all, InstallContext, Progress};
use crate::launch::{launch_atera_debug, launch_atera_debug_with_child, launch_atera_fix_aslr};
use crate::logs::{blob_name_for, build_log_zip, compute_content_digest, upload_blob, LogError};
use crate::manifest::{fetch_manifest, Manifest};
use crate::state::UploadedLedger;
use crate::telemetry::auth::DevSessionRequest;
use crate::telemetry::endpoint::EndpointPolicy;
use crate::telemetry::process_watch::wait_for_exit;
use crate::telemetry::runner::run_session;
use crate::telemetry::Telemetry;

/// Which user-data tree a `WipeClient*` command targets. Internal enum
/// so the public `Command` API splits the two operations into separate
/// variants — easier for the UI to grep for "WipeAll…" when reviewing
/// the destructive paths.
#[derive(Debug, Clone, Copy)]
enum WipeTarget {
    CacheOnly,
    EntireFiresky,
}

pub struct Worker {
    runtime: Arc<Runtime>,
    pub events_rx: mpsc::UnboundedReceiver<Event>,
    events_tx: EventSender,
    /// Cancel token for the currently-running install, if any. Replaced
    /// at the start of every new install (after cancelling the previous
    /// one) so two installs cannot run concurrently and race on temp
    /// zips, extraction, `launcher-installed.json`, and the SGW.exe
    /// hostname patch.
    current_install_cancel: Option<CancellationToken>,
    /// Shared HTTP client reused across seed / patch / manifest fetches
    /// and log uploads. Connection pool persists across requests;
    /// `https_only(true)` defends against http:// downgrade.
    http: reqwest::Client,
    /// Telemetry's own client: the default telemetry server is plain
    /// http on this machine, which `http` refuses. Its address policy
    /// is [`crate::telemetry::endpoint`].
    telemetry_http: reqwest::Client,
    /// The self-updater's client (GitHub hosts only), endpoints and build
    /// identity.
    updater: self_update::Updater,
    /// What is running, so conflicting commands are refused here and not
    /// only greyed out in the UI.
    activity: Activity,
}

impl Worker {
    /// `wake` runs after every event the worker queues; the app passes
    /// one that requests an egui repaint (see [`EventSender`]).
    pub fn new(runtime: Arc<Runtime>, wake: Waker) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let http = reqwest::Client::builder()
            .https_only(true)
            .build()
            .expect("build a rustls HTTP client");
        Self {
            runtime,
            events_rx: rx,
            events_tx: EventSender::new(tx, wake),
            current_install_cancel: None,
            http,
            telemetry_http: crate::telemetry::endpoint::client(),
            updater: self_update::Updater::production(),
            activity: Activity::production(),
        }
    }

    /// Tell the UI a command was not started and why.
    fn refuse(&self, action: Busy, conflict: Conflict) {
        tracing::info!(?action, reason = %conflict, "worker refused a conflicting command");
        let _ = self.events_tx.send(Event::Refused {
            action,
            reason: conflict.to_string(),
        });
    }

    pub fn dispatch(&mut self, cmd: Command) {
        match cmd {
            Command::Cancel => {
                if let Some(token) = self.current_install_cancel.take() {
                    token.cancel();
                }
            }
            Command::Install { config, manifest } => {
                match self.activity.begin_install(&config.install_path) {
                    Ok(()) => self.spawn_install(config, manifest),
                    Err(c) => self.refuse(Busy::Install, c),
                }
            }
            Command::LaunchSgw(req) => match self.activity.begin_launch(&req.install_dir) {
                Ok(()) => self.spawn_launch_sgw(req),
                Err(c) => self.refuse(Busy::Launch, c),
            },
            // The Atera bat starts SGW.exe itself, so the worker cannot
            // follow it: the launch slot is held only until the bat starts,
            // and the process probe guards the running game after that.
            Command::LaunchAteraDebug(dir) => match self.activity.begin_launch(&dir) {
                Ok(()) => {
                    let activity = self.activity.clone();
                    self.spawn_launch("AtreaGameDebug.bat", move || {
                        let r = launch_atera_debug(&dir);
                        activity.game_ended();
                        r
                    })
                }
                Err(c) => self.refuse(Busy::Launch, c),
            },
            // Fix ASLR rewrites SGW.exe; the resets delete the client's
            // per-user files. Neither runs under a running game.
            Command::LaunchAteraFixAslr(dir) => match self.activity.check_idle(&dir) {
                Ok(()) => {
                    self.spawn_launch("AtreaFixASLR.bat", move || launch_atera_fix_aslr(&dir))
                }
                Err(c) => self.refuse(Busy::Files, c),
            },
            Command::UploadLogs {
                install_dir,
                sas_url,
                ledger_path,
            } => self.spawn_upload(install_dir, sas_url, ledger_path),
            Command::AdoptExisting {
                install_dir,
                manifest,
            } => match self.activity.begin_install(&install_dir) {
                Ok(()) => self.spawn_adopt(install_dir, manifest),
                Err(c) => self.refuse(Busy::Install, c),
            },
            Command::OpenInExplorer(dir) => self.spawn_open_folder(dir),
            Command::WipeClientCache | Command::WipeAllClientState => {
                let target = match cmd {
                    Command::WipeClientCache => WipeTarget::CacheOnly,
                    _ => WipeTarget::EntireFiresky,
                };
                match self.activity.check_idle(Path::new("")) {
                    Ok(()) => self.spawn_wipe(target),
                    Err(c) => self.refuse(Busy::Files, c),
                }
            }
            Command::LaunchAteraDebugWithTelemetry {
                install_dir,
                telemetry,
            } => match self.activity.begin_launch(&install_dir) {
                Ok(()) => self.spawn_launch_with_telemetry(install_dir, telemetry),
                Err(c) => self.refuse(Busy::Launch, c),
            },
            Command::CheckForUpdate => self.spawn_update_check(),
            Command::ApplyUpdate(release) => self.spawn_update_apply(release),
        }
    }

    fn spawn_launch_with_telemetry(&self, install_dir: PathBuf, cfg: LaunchTelemetryConfig) {
        let events_tx = self.events_tx.clone();
        let http = self.telemetry_http.clone();
        let activity = self.activity.clone();
        self.runtime.spawn(async move {
            let req = DevSessionRequest {
                install_id: cfg.install_id,
                machine_id: cfg.machine_id,
                branch: cfg.branch,
                git_sha: cfg.git_sha,
                launcher_version: cfg.launcher_version.clone(),
                tags: cfg.tags,
            };
            let telemetry = match Telemetry::start_session(
                &http,
                &cfg.auth_base_url,
                req,
                &install_dir,
                &cfg.launcher_version,
                EndpointPolicy::from_login_servers(
                    cfg.login_server_urls.iter().map(String::as_str),
                ),
            )
            .await
            {
                Ok(t) => Arc::new(t),
                Err(e) => {
                    let msg = format!("auth handshake failed: {e}");
                    error!("telemetry session start failed: {e}");
                    // Game still launches without telemetry — telemetry
                    // is supplementary; never block the play action.
                    let _ = events_tx.send(Event::TelemetrySessionError(msg));
                    launch_legacy_atera(&install_dir, &events_tx);
                    activity.game_ended();
                    return;
                }
            };
            let launched = launch_atera_debug_with_child(&install_dir);
            activity.game_ended();
            let child = match launched {
                Ok(c) => {
                    let _ = events_tx.send(Event::Launched("AtreaGameDebug.bat".into(), c.id()));
                    c
                }
                Err(e) => {
                    let _ = events_tx.send(Event::LaunchError(e.to_string()));
                    return;
                }
            };
            let http_arc = Arc::new(http.clone());
            // The Atera bat starts SGW.exe itself, so the client-patches
            // DLL cannot go in and there is no patch log to summarise.
            let exit = Box::pin(wait_for_exit(child));
            match run_session(telemetry, http_arc, exit, install_dir, cfg.state_dir, None).await {
                Ok(outcome) => {
                    let _ = events_tx.send(Event::TelemetrySessionComplete(outcome));
                }
                Err(e) => {
                    error!("telemetry session error: {e}");
                    let _ = events_tx.send(Event::TelemetrySessionError(e.to_string()));
                }
            }
        });
    }

    fn spawn_adopt(&self, install_dir: PathBuf, manifest: Manifest) {
        let events_tx = self.events_tx.clone();
        let activity = self.activity.clone();
        // Adopt is a single filesystem write — synchronous in the worker
        // task so we don't need a separate progress channel. Keep it on
        // the runtime anyway so a slow-disk write doesn't block the UI.
        self.runtime.spawn(async move {
            let result = adopt_existing_install(&install_dir, &manifest);
            activity.end_install();
            match result {
                Ok(_) => {
                    let _ = events_tx.send(Event::AdoptComplete);
                }
                Err(e) => {
                    error!("adopt failed: {e}");
                    let _ = events_tx.send(Event::AdoptError(format!("{e}")));
                }
            }
        });
    }

    fn spawn_wipe(&self, target: WipeTarget) {
        let events_tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            let (kind, resolved) = match target {
                WipeTarget::CacheOnly => ("Cache.en-US".to_string(), cache_dir()),
                WipeTarget::EntireFiresky => ("Firesky".to_string(), firesky_root()),
            };
            let Some(path) = resolved else {
                let _ = events_tx.send(Event::WipeError(
                    "Could not resolve %USERPROFILE% or $HOME — no user profile to wipe.".into(),
                ));
                return;
            };
            // Recursive remove_dir_all + size walk is blocking IO —
            // offload off the async worker pool so a slow disk doesn't
            // starve concurrent download/upload tasks.
            let result = tokio::task::spawn_blocking(move || wipe_dir_contents(&path)).await;
            match result {
                Ok(Ok(report)) => {
                    let _ = events_tx.send(Event::Wiped { kind, report });
                }
                Ok(Err(e)) => {
                    error!(target = ?target, "wipe failed: {e}");
                    let _ = events_tx.send(Event::WipeError(format!("Wipe {kind} failed: {e}")));
                }
                Err(join_err) => {
                    error!(target = ?target, "wipe task panicked: {join_err}");
                    let _ = events_tx.send(Event::WipeError(format!("Wipe {kind} panicked")));
                }
            }
        });
    }

    pub fn fetch_manifest_now(&self, url: String) {
        let events_tx = self.events_tx.clone();
        let http = self.http.clone();
        self.runtime.spawn(async move {
            let event = match fetch_manifest(&http, &url).await {
                Ok(manifest) => Event::ManifestFetched { url, manifest },
                Err(e) => Event::ManifestError {
                    url,
                    message: e.to_string(),
                },
            };
            let _ = events_tx.send(event);
        });
    }

    /// Called only once [`Activity::begin_install`] has claimed the
    /// install slot, so two installs never race on the .tmp-* zips, the
    /// extract, launcher-installed.json, or the game's files; the task
    /// releases the slot when it ends.
    fn spawn_install(&mut self, config: LauncherConfig, manifest: Manifest) {
        let cancel = CancellationToken::new();
        self.current_install_cancel = Some(cancel.clone());

        let events_tx = self.events_tx.clone();
        let events_tx_for_fwd = self.events_tx.clone();
        let http = self.http.clone();
        let activity = self.activity.clone();
        let (prog_tx, mut prog_rx) = mpsc::unbounded_channel::<Progress>();
        let _ = events_tx.send(Event::InstallStarted);

        // Forwarder: re-emit install Progress as Event::Progress. Loop ends
        // when prog_tx (held inside the install task's InstallContext) drops.
        self.runtime.spawn(async move {
            while let Some(p) = prog_rx.recv().await {
                let _ = events_tx_for_fwd.send(Event::Progress(p));
            }
        });

        self.runtime.spawn(async move {
            let install_dir = config.install_path.clone();
            let ctx = InstallContext {
                manifest_url: &config.manifest_url,
                install_dir: &install_dir,
                manifest: &manifest,
                login_servers: &config.login_servers,
                cancel,
                progress: prog_tx,
                http: &http,
            };
            let (result, report) = install_all(ctx).await;
            activity.end_install();
            crate::telemetry::install_result::queue_install_result(
                &report,
                config.telemetry.opted_in,
                &crate::config::exe_dir(),
            );
            match result {
                Ok(_) => {
                    let _ = events_tx.send(Event::InstallComplete);
                }
                Err(crate::install::InstallError::Cancelled) => {
                    let _ = events_tx.send(Event::InstallCancelled);
                }
                Err(e) => {
                    error!("install failed: {e}");
                    let _ = events_tx.send(Event::InstallError(e.to_string()));
                }
            }
        });
    }

    fn spawn_launch<F>(&self, name: &str, f: F)
    where
        F: FnOnce() -> Result<u32, crate::launch::LaunchError> + Send + 'static,
    {
        let events_tx = self.events_tx.clone();
        let name = name.to_string();
        // Run on the runtime so we don't block the UI thread, even though
        // spawning a child process is cheap.
        self.runtime.spawn(async move {
            match f() {
                Ok(pid) => {
                    let _ = events_tx.send(Event::Launched(name, pid));
                }
                Err(e) => {
                    let _ = events_tx.send(Event::LaunchError(e.to_string()));
                }
            }
        });
    }

    fn spawn_upload(&self, install_dir: PathBuf, sas_url: String, ledger_path: PathBuf) {
        let events_tx = self.events_tx.clone();
        let http = self.http.clone();
        self.runtime.spawn(async move {
            if let Err(e) =
                upload_logs_task(&http, &install_dir, &sas_url, &ledger_path, &events_tx).await
            {
                let _ = events_tx.send(Event::UploadError(e.to_string()));
            }
        });
    }
}

async fn upload_logs_task(
    http: &reqwest::Client,
    install_dir: &std::path::Path,
    sas_url: &str,
    ledger_path: &std::path::Path,
    events_tx: &EventSender,
) -> Result<(), LogError> {
    let digest = match compute_content_digest(install_dir)? {
        Some(d) => d,
        None => {
            let _ = events_tx.send(Event::UploadSkipped("No log files found.".into()));
            return Ok(());
        }
    };

    let mut ledger = UploadedLedger::load(ledger_path);
    if ledger.contains(&digest) {
        let _ = events_tx.send(Event::UploadSkipped(format!(
            "Already uploaded this exact log set (digest {})",
            &digest[..12.min(digest.len())]
        )));
        return Ok(());
    }

    let _ = events_tx.send(Event::UploadStarted);
    let zip_bytes = match build_log_zip(install_dir)? {
        Some(b) => b,
        None => {
            let _ = events_tx.send(Event::UploadSkipped("No log files found.".into()));
            return Ok(());
        }
    };
    let bytes = zip_bytes.len();
    let blob = blob_name_for(&digest);
    upload_blob(http, sas_url, &blob, zip_bytes).await?;
    ledger.record(digest, blob.clone());
    if let Err(e) = ledger.save(ledger_path) {
        error!("failed to persist uploaded ledger: {e}");
    }
    let _ = events_tx.send(Event::UploadComplete { blob, bytes });
    Ok(())
}

/// Fallback launch path when telemetry's auth handshake fails — fire
/// the bat normally so the dev still gets to play. Mirrors
/// `spawn_launch`'s shape.
fn launch_legacy_atera(install_dir: &Path, events_tx: &EventSender) {
    match launch_atera_debug(install_dir) {
        Ok(pid) => {
            let _ = events_tx.send(Event::Launched("AtreaGameDebug.bat".into(), pid));
        }
        Err(e) => {
            let _ = events_tx.send(Event::LaunchError(e.to_string()));
        }
    }
}

#[cfg(test)]
mod guard_tests;

#[cfg(test)]
mod tests;
