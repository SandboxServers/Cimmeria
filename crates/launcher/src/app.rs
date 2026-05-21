//! egui app — top-level UI state machine and panels.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;
use tokio::runtime::Runtime;

use crate::config::{config_path, ledger_path, LauncherConfig, LOG_UPLOAD_SAS_URL};
use crate::install::Progress;
use crate::launch::LaunchOptions;
use crate::manifest::Manifest;
use crate::state::InstalledState;
use crate::worker::{Command, Event, Worker};

/// Upper bound on the status-log history kept in memory. Display already
/// caps at the last 100 entries; this prevents the underlying Vec from
/// growing without bound during long sessions full of events.
const MAX_STATUS_LINES: usize = 1000;

pub struct LauncherApp {
    config: LauncherConfig,
    config_path: PathBuf,
    worker: Worker,
    last_progress: Option<Progress>,
    status: Vec<String>,
    manifest: Option<Manifest>,
    manifest_error: Option<String>,
    installed: InstalledState,
    launch_opts: LaunchOptions,
    last_refresh: std::time::Instant,
    installing: bool,
}

impl LauncherApp {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        let cp = config_path();
        let config = LauncherConfig::load(&cp).unwrap_or_default();
        let worker = Worker::new(runtime);
        let installed = if config.install_path.is_empty() {
            InstalledState::default()
        } else {
            InstalledState::load(std::path::Path::new(&config.install_path))
        };
        let launch_opts = if config.install_path.is_empty() {
            LaunchOptions::default()
        } else {
            LaunchOptions::detect(std::path::Path::new(&config.install_path))
        };
        worker.fetch_manifest_now(config.manifest_url.clone());
        Self {
            config,
            config_path: cp,
            worker,
            last_progress: None,
            status: Vec::new(),
            manifest: None,
            manifest_error: None,
            installed,
            launch_opts,
            last_refresh: std::time::Instant::now(),
            installing: false,
        }
    }

    /// Append a status line, dropping the oldest entries when the buffer
    /// would exceed [`MAX_STATUS_LINES`]. The display only ever reads the
    /// most-recent 100 entries (see [`Self::show_status_log`]) so older
    /// drops are invisible to the user.
    fn push_status(&mut self, line: String) {
        self.status.push(line);
        if self.status.len() > MAX_STATUS_LINES {
            let overflow = self.status.len() - MAX_STATUS_LINES;
            self.status.drain(0..overflow);
        }
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.worker.events_rx.try_recv() {
            match ev {
                Event::ManifestFetched(m) => {
                    self.manifest = Some(m);
                    self.manifest_error = None;
                }
                Event::ManifestError(e) => {
                    self.manifest_error = Some(e);
                }
                Event::Progress(p) => {
                    self.last_progress = Some(p);
                }
                Event::InstallComplete => {
                    self.installing = false;
                    self.push_status("Install complete.".into());
                    self.refresh_install_state();
                }
                Event::InstallError(e) => {
                    self.installing = false;
                    self.push_status(format!("Install failed: {e}"));
                }
                Event::Launched(name, pid) => {
                    self.push_status(format!("Launched {name} (pid {pid})"));
                }
                Event::LaunchError(e) => {
                    self.push_status(format!("Launch failed: {e}"));
                }
                Event::UploadStarted => {
                    self.push_status("Uploading logs…".into());
                }
                Event::UploadSkipped(why) => {
                    self.push_status(format!("Log upload skipped: {why}"));
                }
                Event::UploadComplete { blob, bytes } => {
                    self.push_status(format!("Uploaded {bytes} bytes to {blob}"));
                }
                Event::UploadError(e) => {
                    self.push_status(format!("Log upload failed: {e}"));
                }
            }
            ctx.request_repaint();
        }
    }

    fn refresh_install_state(&mut self) {
        if !self.config.install_path.is_empty() {
            let path = std::path::Path::new(&self.config.install_path);
            self.installed = InstalledState::load(path);
            self.launch_opts = LaunchOptions::detect(path);
        } else {
            self.installed = InstalledState::default();
            self.launch_opts = LaunchOptions::default();
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events(ctx);
        if self.installing {
            // Keep ticking so progress bars animate even when no events
            // happen to arrive between frames.
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        if self.last_refresh.elapsed() > Duration::from_secs(2) {
            self.refresh_install_state();
            self.last_refresh = std::time::Instant::now();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Stargate Worlds Launcher");
            ui.separator();
            self.show_config_panel(ui);
            ui.separator();
            self.show_manifest_summary(ui);
            ui.separator();
            self.show_install_panel(ui);
            ui.separator();
            self.show_launch_panel(ui);
            ui.separator();
            self.show_log_upload_panel(ui);
            ui.separator();
            self.show_status_log(ui);
        });
    }
}

impl LauncherApp {
    fn show_config_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Install dir:");
            ui.add(egui::TextEdit::singleline(&mut self.config.install_path).desired_width(380.0));
            if ui.button("Save").clicked() {
                match self.config.save(&self.config_path) {
                    Ok(_) => {
                        self.push_status("Saved config.".into());
                        self.refresh_install_state();
                    }
                    Err(e) => self.push_status(format!("Save failed: {e}")),
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Server host:");
            ui.add(egui::TextEdit::singleline(&mut self.config.server_host).desired_width(240.0));
            ui.label(
                egui::RichText::new("(patched into SGW.exe .rdata, max 22 bytes)")
                    .small()
                    .italics(),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Manifest URL:");
            ui.add(egui::TextEdit::singleline(&mut self.config.manifest_url).desired_width(380.0));
            if ui.button("Refresh").clicked() {
                self.worker
                    .fetch_manifest_now(self.config.manifest_url.clone());
                self.manifest = None;
                self.manifest_error = None;
            }
        });
    }

    fn show_manifest_summary(&mut self, ui: &mut egui::Ui) {
        match (&self.manifest, &self.manifest_error) {
            (Some(m), _) => {
                ui.label(format!(
                    "Manifest schema {}, seed {} ({} bytes), {} patch(es) declared.",
                    m.schema,
                    m.seed.blob,
                    m.seed.size,
                    m.patches.len()
                ));
            }
            (None, Some(e)) => {
                ui.colored_label(egui::Color32::RED, format!("Manifest error: {e}"));
            }
            (None, None) => {
                ui.label("Fetching manifest…");
            }
        }
    }

    fn show_install_panel(&mut self, ui: &mut egui::Ui) {
        let Some(manifest) = self.manifest.clone() else {
            return;
        };
        if self.config.install_path.is_empty() {
            ui.label("Set an install directory to enable install / update.");
            return;
        }

        let seed_ok = self.installed.seed_sha256.as_deref() == Some(manifest.seed.sha256.as_str());
        let missing_patches: Vec<String> = manifest
            .patches
            .iter()
            .filter(|p| !self.installed.has_applied(&p.id))
            .map(|p| p.id.clone())
            .collect();

        // The hostname patch lives outside the seed/patch model: it's a
        // post-install step that runs on `Install / Update`. If the user
        // edits `server_host` after a complete install, neither seed nor
        // patches change but we still want to surface the re-patch action.
        let host_needs_repatch = !self.config.server_host.is_empty()
            && self.installed.patched_host.as_deref() != Some(self.config.server_host.as_str());

        if seed_ok && missing_patches.is_empty() && !host_needs_repatch {
            ui.colored_label(egui::Color32::LIGHT_GREEN, "✔ Install is up to date.");
            return;
        }

        if !seed_ok {
            ui.label("Seed not installed (or hash mismatch) — will download seed first.");
        }
        if !missing_patches.is_empty() {
            ui.label(format!(
                "Missing patches ({}): {}",
                missing_patches.len(),
                missing_patches.join(", ")
            ));
        }
        if seed_ok && missing_patches.is_empty() && host_needs_repatch {
            ui.label(format!(
                "Server host changed to '{}' — SGW.exe needs re-patching.",
                self.config.server_host
            ));
        }

        let installing = self.installing;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!installing, egui::Button::new("Install / Update"))
                .clicked()
            {
                if let Err(e) = self.config.save(&self.config_path) {
                    self.push_status(format!("Save failed: {e}"));
                }
                self.installing = true;
                self.last_progress = None;
                self.push_status("Starting install / update…".into());
                self.worker.dispatch(Command::Install {
                    config: self.config.clone(),
                    manifest: manifest.clone(),
                });
            }
            if ui
                .add_enabled(installing, egui::Button::new("Cancel"))
                .clicked()
            {
                self.worker.dispatch(Command::Cancel);
                self.push_status("Cancel requested.".into());
            }
        });
        self.show_progress(ui);
    }

    fn show_progress(&self, ui: &mut egui::Ui) {
        let Some(p) = &self.last_progress else { return };
        match p {
            Progress::Downloading {
                label,
                downloaded,
                total,
            } => {
                let pct = if *total > 0 {
                    (*downloaded as f32 / *total as f32).min(1.0)
                } else {
                    0.0
                };
                ui.label(format!(
                    "{}: {} / {} bytes",
                    label,
                    human_bytes(*downloaded),
                    human_bytes(*total)
                ));
                ui.add(egui::ProgressBar::new(pct).show_percentage());
            }
            Progress::Extracting {
                label,
                current,
                total,
                filename,
            } => {
                let pct = if *total > 0 {
                    (*current as f32 / *total as f32).min(1.0)
                } else {
                    0.0
                };
                ui.label(format!("{label}: {current} / {total} — {filename}"));
                ui.add(egui::ProgressBar::new(pct).show_percentage());
            }
        }
    }

    fn show_launch_panel(&mut self, ui: &mut egui::Ui) {
        let dir = PathBuf::from(&self.config.install_path);
        let opts = self.launch_opts.clone();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(opts.sgw_present, egui::Button::new("Launch SGW.exe"))
                .clicked()
            {
                self.worker.dispatch(Command::LaunchSgw(dir.clone()));
            }
            if ui
                .add_enabled(
                    opts.atera_available(),
                    egui::Button::new("Launch Atera Debug"),
                )
                .clicked()
            {
                self.worker.dispatch(Command::LaunchAteraDebug(dir.clone()));
            }
            if ui
                .add_enabled(
                    opts.atera_fix_aslr_bat_present,
                    egui::Button::new("Fix ASLR"),
                )
                .clicked()
            {
                self.worker
                    .dispatch(Command::LaunchAteraFixAslr(dir.clone()));
            }
        });
        if !opts.atera_available() {
            ui.label(
                egui::RichText::new(
                    "Atera debug requires AteraLoader.exe + AtreaGameDebug.bat alongside SGW.exe. \
                     Atera debug also requires ASLR disabled on SGW.exe — run \"Fix ASLR\" once first.",
                )
                .small()
                .italics(),
            );
        }
    }

    fn show_log_upload_panel(&mut self, ui: &mut egui::Ui) {
        let sas = LOG_UPLOAD_SAS_URL;
        let dir = PathBuf::from(&self.config.install_path);

        ui.horizontal(|ui| {
            let enabled = sas.is_some() && !self.config.install_path.is_empty();
            if ui
                .add_enabled(enabled, egui::Button::new("Upload Debug Logs"))
                .clicked()
            {
                if let Some(sas) = sas {
                    self.worker.dispatch(Command::UploadLogs {
                        install_dir: dir,
                        sas_url: sas.to_string(),
                        ledger_path: ledger_path(),
                    });
                }
            }
            if sas.is_none() {
                ui.label(
                    egui::RichText::new(
                        "Log upload disabled — this build has no LAUNCHER_LOG_SAS_URL baked in.",
                    )
                    .small()
                    .italics(),
                );
            }
        });
        ui.label(
            egui::RichText::new(
                "Zips Binaries/sgwdebuglog* + Binaries/sessions/** into one file and uploads it. \
                 Already-uploaded log sets dedupe locally — no double-billing on repeat clicks.",
            )
            .small(),
        );
    }

    fn show_status_log(&self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Status").strong());
        egui::ScrollArea::vertical()
            .max_height(160.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in self
                    .status
                    .iter()
                    .rev()
                    .take(100)
                    .collect::<Vec<_>>()
                    .iter()
                    .rev()
                {
                    ui.label(line.as_str());
                }
            });
    }
}

fn human_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut f = n as f64;
    let mut i = 0;
    while f >= 1024.0 && i + 1 < UNITS.len() {
        f /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{f:.2} {}", UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::{human_bytes, MAX_STATUS_LINES};

    #[test]
    fn human_bytes_formats_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(2048), "2.00 KB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.00 MB");
    }

    // Drives the same drain-on-overflow shape that `push_status` uses, with
    // a Vec we can inspect directly. Keeps the test free of the full
    // LauncherApp construction (which requires a tokio runtime).
    fn push_capped(buf: &mut Vec<String>, line: String) {
        buf.push(line);
        if buf.len() > MAX_STATUS_LINES {
            let overflow = buf.len() - MAX_STATUS_LINES;
            buf.drain(0..overflow);
        }
    }

    #[test]
    fn push_status_caps_at_max_lines() {
        let mut buf = Vec::new();
        for i in 0..(MAX_STATUS_LINES + 25) {
            push_capped(&mut buf, format!("line {i}"));
        }
        assert_eq!(buf.len(), MAX_STATUS_LINES);
        // Oldest 25 should have been dropped.
        assert_eq!(buf.first().unwrap(), "line 25");
        assert_eq!(
            buf.last().unwrap(),
            &format!("line {}", MAX_STATUS_LINES + 24)
        );
    }

    #[test]
    fn push_status_under_cap_does_not_drain() {
        let mut buf = Vec::new();
        for i in 0..10 {
            push_capped(&mut buf, format!("{i}"));
        }
        assert_eq!(buf.len(), 10);
        assert_eq!(buf.first().unwrap(), "0");
    }
}
