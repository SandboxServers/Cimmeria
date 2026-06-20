//! egui panel rendering for [`LauncherApp`].
//!
//! Holds the `eframe::App` frame entry point and every `show_*` panel
//! method. The state struct, lifecycle, and pure helpers live in the
//! parent [`module`](super).

use std::time::Duration;

use eframe::egui;

use super::{
    build_telemetry_config, human_bytes, path_is_empty, should_show_adopt_button, LauncherApp,
};
use crate::config::{ledger_path, LOG_UPLOAD_SAS_URL};
use crate::install::Progress;
use crate::launch::install_dir_writable;
use crate::worker::Command;

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.drain_events(&ctx);
        if self.installing {
            // Keep ticking so progress bars animate even when no events
            // happen to arrive between frames.
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        if self.last_refresh.elapsed() > Duration::from_secs(2) {
            self.refresh_install_state();
            self.last_refresh = std::time::Instant::now();
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
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
        self.show_confirm_wipe_all_modal(&ctx);
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
            // Telemetry-enabled launch needs identity + opt-in + atera
            // available. Falls back to plain Launch Atera Debug
            // (legacy) when telemetry is opted out or identity load
            // failed.
            let telemetry_ready =
                opts.atera_available() && self.config.telemetry.enabled && self.identity.is_some();
            if ui
                .add_enabled(telemetry_ready, egui::Button::new("Launch + Telemetry"))
                .clicked()
            {
                if let Some(id) = &self.identity {
                    self.worker
                        .dispatch(Command::LaunchAteraDebugWithTelemetry {
                            install_dir: dir.clone(),
                            telemetry: build_telemetry_config(&self.config, id),
                        });
                }
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
