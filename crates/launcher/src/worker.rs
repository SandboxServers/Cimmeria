//! Background worker.
//!
//! Hosts the tokio runtime and turns UI [`Command`]s into spawned tasks
//! that emit [`Event`]s back on an unbounded channel. The egui app
//! polls the channel each frame via `events_rx.try_recv()`.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::client_paths::{cache_dir, firesky_root, wipe_dir_contents, WipeReport};
use crate::config::LauncherConfig;
use crate::install::{adopt_existing_install, install_all, InstallContext, Progress};
use crate::launch::{launch_atera_debug, launch_atera_fix_aslr, launch_sgw};
use crate::logs::{blob_name_for, build_log_zip, compute_content_digest, upload_blob, LogError};
use crate::manifest::{fetch_manifest, Manifest};
use crate::state::UploadedLedger;

#[derive(Debug, Clone)]
pub enum Command {
    Install {
        config: LauncherConfig,
        manifest: Manifest,
    },
    /// Mark a pre-existing game install as managed by the launcher
    /// without re-downloading the seed. See [`adopt_existing_install`].
    AdoptExisting {
        install_dir: PathBuf,
        manifest: Manifest,
    },
    LaunchSgw(PathBuf),
    LaunchAteraDebug(PathBuf),
    LaunchAteraFixAslr(PathBuf),
    UploadLogs {
        install_dir: PathBuf,
        sas_url: String,
        ledger_path: PathBuf,
    },
    /// Wipe `Documents\My Games\Firesky\SGWGame\Cache.en-US\` — the
    /// server-pushed PAK override cache. Safe to run any time the user
    /// wants the launcher-managed PAKs to win over previously-cached
    /// server pushes.
    WipeClientCache,
    /// Wipe the full `Documents\My Games\Firesky\` tree — the recovery
    /// recipe for cache corruption per docs/client-tools.md. Higher
    /// blast radius (also nukes per-user settings, keybinds, screenshots
    /// if any) — UI must confirm-dialog gate this.
    WipeAllClientState,
    Cancel,
}

#[derive(Debug, Clone)]
pub enum Event {
    ManifestFetched(Manifest),
    ManifestError(String),
    Progress(Progress),
    InstallComplete,
    InstallError(String),
    AdoptComplete,
    AdoptError(String),
    /// Reports per-second visible feedback after the wipe finishes —
    /// `kind` is "Cache.en-US" vs "Firesky" so the UI can render the
    /// right caption.
    Wiped {
        kind: String,
        report: WipeReport,
    },
    WipeError(String),
    Launched(String, u32),
    LaunchError(String),
    UploadStarted,
    UploadSkipped(String),
    UploadComplete { blob: String, bytes: usize },
    UploadError(String),
}

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
    events_tx: mpsc::UnboundedSender<Event>,
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
}

impl Worker {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let http = reqwest::Client::builder()
            .https_only(true)
            .build()
            .expect("build a rustls HTTP client");
        Self {
            runtime,
            events_rx: rx,
            events_tx: tx,
            current_install_cancel: None,
            http,
        }
    }

    pub fn dispatch(&mut self, cmd: Command) {
        match cmd {
            Command::Cancel => {
                if let Some(token) = self.current_install_cancel.take() {
                    token.cancel();
                }
            }
            Command::Install { config, manifest } => self.spawn_install(config, manifest),
            Command::LaunchSgw(dir) => self.spawn_launch("SGW.exe", move || launch_sgw(&dir)),
            Command::LaunchAteraDebug(dir) => {
                self.spawn_launch("AtreaGameDebug.bat", move || launch_atera_debug(&dir))
            }
            Command::LaunchAteraFixAslr(dir) => {
                self.spawn_launch("AtreaFixASLR.bat", move || launch_atera_fix_aslr(&dir))
            }
            Command::UploadLogs {
                install_dir,
                sas_url,
                ledger_path,
            } => self.spawn_upload(install_dir, sas_url, ledger_path),
            Command::AdoptExisting {
                install_dir,
                manifest,
            } => self.spawn_adopt(install_dir, manifest),
            Command::WipeClientCache => self.spawn_wipe(WipeTarget::CacheOnly),
            Command::WipeAllClientState => self.spawn_wipe(WipeTarget::EntireFiresky),
        }
    }

    fn spawn_adopt(&self, install_dir: PathBuf, manifest: Manifest) {
        let events_tx = self.events_tx.clone();
        // Adopt is a single filesystem write — synchronous in the worker
        // task so we don't need a separate progress channel. Keep it on
        // the runtime anyway so a slow-disk write doesn't block the UI.
        self.runtime.spawn(async move {
            match adopt_existing_install(&install_dir, &manifest) {
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
                    "Could not resolve %USERPROFILE% or $HOME — no user profile to wipe."
                        .into(),
                ));
                return;
            };
            match wipe_dir_contents(&path) {
                Ok(report) => {
                    let _ = events_tx.send(Event::Wiped { kind, report });
                }
                Err(e) => {
                    error!(target = ?target, "wipe failed: {e}");
                    let _ = events_tx.send(Event::WipeError(format!("Wipe {kind} failed: {e}")));
                }
            }
        });
    }

    pub fn fetch_manifest_now(&self, url: String) {
        let events_tx = self.events_tx.clone();
        let http = self.http.clone();
        self.runtime.spawn(async move {
            match fetch_manifest(&http, &url).await {
                Ok(m) => {
                    let _ = events_tx.send(Event::ManifestFetched(m));
                }
                Err(e) => {
                    let _ = events_tx.send(Event::ManifestError(e.to_string()));
                }
            }
        });
    }

    fn spawn_install(&mut self, config: LauncherConfig, manifest: Manifest) {
        // Cancel any previous install before starting a new one. Without
        // this, a rapid double-click on Install / Update spawns two
        // concurrent install tasks that race on the .tmp-* zips, the
        // extract, launcher-installed.json, and the SGW.exe patch. The
        // previous task observes its token flip to cancelled at the
        // next progress checkpoint and bails with InstallError::Cancelled.
        if let Some(prev) = self.current_install_cancel.take() {
            prev.cancel();
        }
        let cancel = CancellationToken::new();
        self.current_install_cancel = Some(cancel.clone());

        let events_tx = self.events_tx.clone();
        let events_tx_for_fwd = self.events_tx.clone();
        let http = self.http.clone();
        let (prog_tx, mut prog_rx) = mpsc::unbounded_channel::<Progress>();

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
                server_host: &config.server_host,
                cancel,
                progress: prog_tx,
                http: &http,
            };
            match install_all(ctx).await {
                Ok(_) => {
                    let _ = events_tx.send(Event::InstallComplete);
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
    events_tx: &mpsc::UnboundedSender<Event>,
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
