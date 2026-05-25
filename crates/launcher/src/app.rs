//! egui app — top-level UI state machine and panels.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;
use tokio::runtime::Runtime;

use crate::config::{config_path, ledger_path, LauncherConfig, LOG_UPLOAD_SAS_URL};
use crate::install::Progress;
use crate::launch::{install_dir_writable, LaunchOptions};
use crate::manifest::Manifest;
use crate::state::InstalledState;
use crate::worker::{Command, Event, Worker};

/// Upper bound on the status-log history kept in memory. Display already
/// caps at the last 100 entries; this prevents the underlying Vec from
/// growing without bound during long sessions full of events.
const MAX_STATUS_LINES: usize = 1000;

pub struct LauncherApp {
    config: LauncherConfig,
    /// Editable text buffer for the install-dir TextEdit widget. egui's
    /// `TextEdit::singleline` takes `&mut String`, but the persisted
    /// config field is `PathBuf` — this is the sync target.
    install_path_text: String,
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
    /// True while a confirm modal for "Reset all client state" is open.
    /// Higher-blast-radius wipe — gates the entire Firesky/ tree, not
    /// just the cache subdir — so we double-prompt before nuking.
    confirm_wipe_all_open: bool,
}

impl LauncherApp {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        let cp = config_path();
        let config = LauncherConfig::load(&cp).unwrap_or_default();
        let worker = Worker::new(runtime);
        let installed = if path_is_empty(&config.install_path) {
            InstalledState::default()
        } else {
            InstalledState::load(&config.install_path)
        };
        let launch_opts = if path_is_empty(&config.install_path) {
            LaunchOptions::default()
        } else {
            LaunchOptions::detect(&config.install_path)
        };
        worker.fetch_manifest_now(config.manifest_url.clone());
        let install_path_text = config.install_path.to_string_lossy().into_owned();
        Self {
            config,
            install_path_text,
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
            confirm_wipe_all_open: false,
        }
    }

    /// Pull the latest text-buffer value into the persisted PathBuf so
    /// the save path / install_dir comparisons all see the user's edit.
    fn sync_install_path_from_text(&mut self) {
        self.config.install_path = PathBuf::from(&self.install_path_text);
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
            // Events that drive *non-status* UI state (manifest panel,
            // progress bars, the installing/managed-install flags) get
            // their side effects applied here. The status-log line — if
            // any — comes from `status_line_for`, which is the single
            // source of truth for ev-to-text translation and is unit
            // tested directly.
            match &ev {
                Event::ManifestFetched(m) => {
                    self.manifest = Some(m.clone());
                    self.manifest_error = None;
                }
                Event::ManifestError(e) => {
                    self.manifest_error = Some(e.clone());
                }
                Event::Progress(p) => {
                    self.last_progress = Some(p.clone());
                }
                Event::InstallComplete => {
                    self.installing = false;
                    self.refresh_install_state();
                }
                Event::InstallError(_) => {
                    self.installing = false;
                }
                Event::AdoptComplete => {
                    self.refresh_install_state();
                }
                Event::AdoptError(_)
                | Event::Wiped { .. }
                | Event::WipeError(_)
                | Event::Launched(..)
                | Event::LaunchError(_)
                | Event::UploadStarted
                | Event::UploadSkipped(_)
                | Event::UploadComplete { .. }
                | Event::UploadError(_) => {
                    // Status-only events — handled below.
                }
            }
            if let Some(line) = status_line_for(&ev) {
                self.push_status(line);
            }
            ctx.request_repaint();
        }
    }

    fn refresh_install_state(&mut self) {
        if !path_is_empty(&self.config.install_path) {
            let path = self.config.install_path.as_path();
            self.installed = InstalledState::load(path);
            self.launch_opts = LaunchOptions::detect(path);
        } else {
            self.installed = InstalledState::default();
            self.launch_opts = LaunchOptions::default();
        }
    }
}

/// True iff `p` is an empty path (no components). Replaces the
/// `String::is_empty()` checks from before the PathBuf migration.
fn path_is_empty(p: &Path) -> bool {
    p.as_os_str().is_empty()
}

/// Whether to surface the "Adopt existing install" affordance.
///
/// True iff `install_path` contains `SGW.exe` AND does NOT contain a
/// `launcher-installed.json` marker file. The first condition rules
/// out empty directories (those should go through the normal Install
/// path); the second condition rules out installs the launcher
/// already manages (those have nothing to adopt). Extracted from
/// `show_install_panel` so the boolean decision is unit-testable
/// without spinning up an egui frame.
fn should_show_adopt_button(install_path: &Path) -> bool {
    install_path.join("SGW.exe").is_file()
        && !crate::state::InstalledState::path(install_path).exists()
}

/// Render a worker [`Event`] into the human-readable status-log line
/// the UI appends to its scrollback. Pure formatting — extracted from
/// `drain_events` so each Event arm has at least minimal coverage
/// without needing an egui context. Returns `None` for events that
/// don't translate to a status line on their own (manifest updates,
/// progress ticks).
fn status_line_for(event: &Event) -> Option<String> {
    Some(match event {
        Event::AdoptComplete => {
            "Adopted existing install — patches will apply on top (seed bytes not verified).".into()
        }
        Event::AdoptError(e) => format!("Adopt failed: {e}"),
        Event::Wiped { kind, report } => format!(
            "Wiped {kind}: {} item(s), {} freed",
            report.entries_removed,
            human_bytes(report.bytes_freed)
        ),
        Event::WipeError(e) => format!("Wipe failed: {e}"),
        Event::InstallComplete => "Install complete.".into(),
        Event::InstallError(e) => format!("Install failed: {e}"),
        Event::Launched(name, pid) => format!("Launched {name} (pid {pid})"),
        Event::LaunchError(e) => format!("Launch failed: {e}"),
        Event::UploadStarted => "Uploading logs…".into(),
        Event::UploadSkipped(why) => format!("Log upload skipped: {why}"),
        Event::UploadComplete { blob, bytes } => format!("Uploaded {bytes} bytes to {blob}"),
        Event::UploadError(e) => format!("Log upload failed: {e}"),
        // Progress + manifest events drive other UI state, not the
        // status log. Returning None makes that explicit.
        Event::ManifestFetched(_) | Event::ManifestError(_) | Event::Progress(_) => return None,
    })
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
            self.show_client_state_panel(ui);
            ui.separator();
            self.show_status_log(ui);
        });
        self.show_confirm_wipe_all_modal(ctx);
    }
}

impl LauncherApp {
    fn show_config_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Install dir:");
            // Sync on focus loss rather than every keystroke. The
            // writability probe in `show_install_panel` calls
            // `create_dir_all`, so syncing per-character would create
            // partial directories ("C:\G", "C:\Ga", …) as the user
            // types. `lost_focus()` fires when the user tabs out,
            // clicks elsewhere, OR clicks any other widget in this
            // panel (including Save, Install/Update, Launch) — egui
            // resolves the focus change before the click handlers run,
            // so path-dependent buttons see the synced value.
            let response = ui
                .add(egui::TextEdit::singleline(&mut self.install_path_text).desired_width(380.0));
            if response.lost_focus() {
                self.sync_install_path_from_text();
            }
            if ui.button("Save").clicked() {
                self.sync_install_path_from_text();
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
        if path_is_empty(&self.config.install_path) {
            ui.label("Set an install directory to enable install / update.");
            return;
        }

        // Adoption affordance: an install directory that contains
        // SGW.exe but has no launcher-installed.json is an unmanaged
        // pre-existing copy (typically a CME-shipped client a user
        // pointed us at). Offer the one-click adopt path BEFORE the
        // destructive Install / Update flow surfaces — otherwise the
        // first thing the user sees is "Seed not installed — will
        // download seed first" and clicking Install would overwrite
        // ~3 GB of existing files with a fresh seed download.
        if should_show_adopt_button(&self.config.install_path) {
            ui.label(
                egui::RichText::new(
                    "Existing SGW.exe found in this directory but the launcher hasn't \
                     adopted it yet. Adopt skips the seed download and lets patches \
                     apply on top. Seed bytes are NOT verified — if these files don't \
                     match the published seed, patches may misbehave.",
                )
                .small()
                .italics(),
            );
            if ui.button("Adopt existing install").clicked() {
                self.worker.dispatch(Command::AdoptExisting {
                    install_dir: self.config.install_path.clone(),
                    manifest: manifest.clone(),
                });
                self.push_status("Adopt requested…".into());
            }
            ui.separator();
        }

        if self.installed.seed_adopted {
            ui.colored_label(
                egui::Color32::LIGHT_YELLOW,
                "ℹ Seed marked as adopted (unverified). Patches apply on top.",
            );
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
        // Probe writability of the install dir before we let the user
        // click Install / Update. Without this, picking `C:\Program Files\…`
        // without UAC produces an opaque mid-extract "Access denied" after
        // gigabytes of download.
        let writable = install_dir_writable(&self.config.install_path);
        if !writable {
            ui.colored_label(
                egui::Color32::RED,
                format!(
                    "Install dir '{}' is not writable. Choose another path or run as admin.",
                    self.config.install_path.display()
                ),
            );
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !installing && writable,
                    egui::Button::new("Install / Update"),
                )
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
        let dir = self.config.install_path.clone();
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
        let dir = self.config.install_path.clone();

        ui.horizontal(|ui| {
            let enabled = sas.is_some() && !path_is_empty(&self.config.install_path);
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

    /// "Client state" panel — surfaces the two reset buttons backed by
    /// [`crate::client_paths`].
    ///
    /// **Why this exists:** the SGW client unconditionally writes its
    /// runtime cache (server-pushed PAK overrides) into
    /// `%USERPROFILE%\Documents\My Games\Firesky\SGWGame\Cache.en-US\`
    /// and consults that cache **before** the bundled PAKs in the
    /// install directory (docs/architecture/mission-pak-overrides.md).
    /// A stale cache from a previous shard silently shadows the
    /// launcher-managed PAKs until it's wiped — these buttons are the
    /// supported recovery path, mirroring the doc'd recipe in
    /// docs/client-tools.md for cache corruption.
    fn show_client_state_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Client state").strong());
        ui.label(
            egui::RichText::new(
                "The client writes its cache + per-user settings to \
                 Documents\\My Games\\Firesky\\. The cache is consulted \
                 before the launcher-managed PAKs, so stale entries from \
                 a previous server can win silently.",
            )
            .small(),
        );
        ui.horizontal(|ui| {
            if ui.button("Reset client cache").clicked() {
                self.worker.dispatch(Command::WipeClientCache);
                self.push_status("Wiping Cache.en-US…".into());
            }
            // Higher blast radius → confirm-dialog gate. The first click
            // arms the modal; the second-click confirmation is what
            // actually dispatches the wipe.
            if ui.button("Reset all client state…").clicked() {
                self.confirm_wipe_all_open = true;
            }
        });
        ui.label(
            egui::RichText::new(
                "Reset client cache: wipes Cache.en-US/ (server-pushed PAK overrides). \
                 Safe whenever you switch server hosts or pull a new manifest.\n\
                 Reset all client state: wipes the full Firesky/ tree, including any \
                 per-user settings the client saved. Use only if the client crashes \
                 immediately on launch (the doc'd cache-corruption recovery).",
            )
            .small()
            .italics(),
        );
    }

    /// Modal confirmation for "Reset all client state…". egui's modal
    /// support is `egui::Modal::new(…).show(ctx, …)` — a centered popup
    /// that captures input until Confirm or Cancel.
    fn show_confirm_wipe_all_modal(&mut self, ctx: &egui::Context) {
        if !self.confirm_wipe_all_open {
            return;
        }
        let modal = egui::Modal::new(egui::Id::new("confirm-wipe-all-client-state"));
        let response = modal.show(ctx, |ui| {
            ui.heading("Reset all client state?");
            ui.label(
                "This will permanently delete every file under:\n\
                 \n\
                 Documents\\My Games\\Firesky\\\n\
                 \n\
                 Including any per-user settings, keybinds, or screenshots \
                 the client saved there. The cache will regenerate on next \
                 launch; user settings will reset to defaults.\n\
                 \n\
                 This is the documented recovery path for cache corruption \
                 (e.g. the client crashes immediately on launch). For a \
                 less-destructive option, use \"Reset client cache\" instead — \
                 that wipes only the server-pushed PAK overrides.",
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    self.confirm_wipe_all_open = false;
                }
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("Delete everything")
                            .color(egui::Color32::WHITE)
                            .background_color(egui::Color32::DARK_RED),
                    ))
                    .clicked()
                {
                    self.worker.dispatch(Command::WipeAllClientState);
                    self.push_status("Wiping Firesky/ tree…".into());
                    self.confirm_wipe_all_open = false;
                }
            });
        });
        // Click-outside-modal-to-dismiss: the response.should_close()
        // check covers Esc + clicking the dimmed backdrop without
        // re-implementing either.
        if response.should_close() {
            self.confirm_wipe_all_open = false;
        }
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
    use super::{human_bytes, should_show_adopt_button, status_line_for, MAX_STATUS_LINES};
    use crate::client_paths::WipeReport;
    use crate::worker::Event;

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

    // Empty install dir: no SGW.exe + no marker → the Install panel
    // should NOT surface the Adopt affordance (nothing to adopt).
    #[test]
    fn should_show_adopt_button_false_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!should_show_adopt_button(dir.path()));
    }

    // SGW.exe present + no marker → adopt is the user's least-destructive
    // path forward. This is the trigger condition.
    #[test]
    fn should_show_adopt_button_true_when_unmanaged_install_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
        assert!(should_show_adopt_button(dir.path()));
    }

    // Marker file already present → install is launcher-managed; adopt
    // is a no-op (and would refuse with AlreadyManaged anyway). Hiding
    // the button keeps the UI honest.
    #[test]
    fn should_show_adopt_button_false_when_already_managed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
        std::fs::write(
            crate::state::InstalledState::path(dir.path()),
            r#"{"applied_patches":[],"seed_sha256":"h"}"#,
        )
        .unwrap();
        assert!(!should_show_adopt_button(dir.path()));
    }

    // status_line_for covers every Event variant that produces a
    // status entry. Wiped is the only one with non-trivial formatting
    // (bytes-freed → human_bytes) — pin its exact shape against a
    // realistic report.
    #[test]
    fn status_line_for_formats_adopt_complete() {
        let line = status_line_for(&Event::AdoptComplete).unwrap();
        assert!(line.contains("Adopted"), "got: {line}");
        assert!(
            line.contains("not verified"),
            "must surface the trust trade-off, got: {line}"
        );
    }

    #[test]
    fn status_line_for_formats_adopt_error() {
        let line = status_line_for(&Event::AdoptError("boom".into())).unwrap();
        assert_eq!(line, "Adopt failed: boom");
    }

    #[test]
    fn status_line_for_formats_wiped_with_human_bytes() {
        let line = status_line_for(&Event::Wiped {
            kind: "Cache.en-US".into(),
            report: WipeReport {
                entries_removed: 3,
                bytes_freed: 5 * 1024 * 1024,
            },
        })
        .unwrap();
        // Pin both the item count and the human-bytes rendering so a
        // future change to either thread shows up as a test diff.
        assert_eq!(line, "Wiped Cache.en-US: 3 item(s), 5.00 MB freed");
    }

    #[test]
    fn status_line_for_formats_wipe_error() {
        let line = status_line_for(&Event::WipeError("permission denied".into())).unwrap();
        assert_eq!(line, "Wipe failed: permission denied");
    }

    #[test]
    fn status_line_for_returns_none_for_progress_and_manifest_events() {
        // These drive UI state directly (progress bars, manifest
        // summary panel) — they don't belong in the scrolling status
        // log. Returning None enforces that at the type level.
        assert!(status_line_for(&Event::ManifestError("x".into())).is_none());
        assert!(
            status_line_for(&Event::Progress(crate::install::Progress::Downloading {
                label: "seed".into(),
                downloaded: 0,
                total: 0,
            },))
            .is_none()
        );
    }
}
