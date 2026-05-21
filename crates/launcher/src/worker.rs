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

use crate::config::LauncherConfig;
use crate::install::{install_all, InstallContext, Progress};
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
    LaunchSgw(PathBuf),
    LaunchAteraDebug(PathBuf),
    LaunchAteraFixAslr(PathBuf),
    UploadLogs {
        install_dir: PathBuf,
        sas_url: String,
        ledger_path: PathBuf,
    },
    Cancel,
}

#[derive(Debug, Clone)]
pub enum Event {
    ManifestFetched(Manifest),
    ManifestError(String),
    Progress(Progress),
    InstallComplete,
    InstallError(String),
    Launched(String, u32),
    LaunchError(String),
    UploadStarted,
    UploadSkipped(String),
    UploadComplete { blob: String, bytes: usize },
    UploadError(String),
}

pub struct Worker {
    runtime: Arc<Runtime>,
    pub events_rx: mpsc::UnboundedReceiver<Event>,
    events_tx: mpsc::UnboundedSender<Event>,
    cancel: CancellationToken,
}

impl Worker {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            runtime,
            events_rx: rx,
            events_tx: tx,
            cancel: CancellationToken::new(),
        }
    }

    pub fn dispatch(&mut self, cmd: Command) {
        match cmd {
            Command::Cancel => {
                self.cancel.cancel();
                self.cancel = CancellationToken::new();
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
        }
    }

    pub fn fetch_manifest_now(&self, url: String) {
        let events_tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            match fetch_manifest(&url).await {
                Ok(m) => {
                    let _ = events_tx.send(Event::ManifestFetched(m));
                }
                Err(e) => {
                    let _ = events_tx.send(Event::ManifestError(e.to_string()));
                }
            }
        });
    }

    fn spawn_install(&self, config: LauncherConfig, manifest: Manifest) {
        let cancel = self.cancel.clone();
        let events_tx = self.events_tx.clone();
        let events_tx_for_fwd = self.events_tx.clone();
        let (prog_tx, mut prog_rx) = mpsc::unbounded_channel::<Progress>();

        // Forwarder: re-emit install Progress as Event::Progress. Loop ends
        // when prog_tx (held inside the install task's InstallContext) drops.
        self.runtime.spawn(async move {
            while let Some(p) = prog_rx.recv().await {
                let _ = events_tx_for_fwd.send(Event::Progress(p));
            }
        });

        self.runtime.spawn(async move {
            let install_dir = PathBuf::from(&config.install_path);
            let ctx = InstallContext {
                manifest_url: &config.manifest_url,
                install_dir: &install_dir,
                manifest: &manifest,
                server_host: &config.server_host,
                cancel,
                progress: prog_tx,
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
        self.runtime.spawn(async move {
            if let Err(e) = upload_logs_task(&install_dir, &sas_url, &ledger_path, &events_tx).await
            {
                let _ = events_tx.send(Event::UploadError(e.to_string()));
            }
        });
    }
}

async fn upload_logs_task(
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
    upload_blob(sas_url, &blob, zip_bytes).await?;
    ledger.record(digest, blob.clone());
    if let Err(e) = ledger.save(ledger_path) {
        error!("failed to persist uploaded ledger: {e}");
    }
    let _ = events_tx.send(Event::UploadComplete { blob, bytes });
    Ok(())
}
