//! Settings › Advanced: every configuration and debug tool the launcher
//! had before the redesign, moved out of the player's way.
//!
//! Login servers, the manifest URL, manual Install / Update and adoption,
//! the client-patches opt-out, the Atera debug launches and Fix ASLR, log
//! upload, the client-state resets, launcher update controls, the
//! "Changes to your client" disclosure, and the activity log. Their
//! warnings, confirmations and availability rules are unchanged; the
//! ones that touch the install's files also wait for the game to close.

use eframe::egui::{self, RichText};

use super::play_state::{file_action_block, launch_block};
use super::{build_telemetry_config, path_is_empty, should_show_adopt_button, theme, LauncherApp};
use crate::config::{ledger_path, LOG_UPLOAD_SAS_URL};
use crate::worker::Command;

impl LauncherApp {
    pub(super) fn show_advanced(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(RichText::new("Advanced").strong())
            .id_salt("advanced")
            .show(ui, |ui| {
                ui.label(theme::muted(
                    "Configuration and troubleshooting tools. Most players never need these.",
                ));
                section(ui, "Login servers", |ui| self.show_login_servers(ui));
                section(ui, "Content manifest", |ui| self.show_manifest_source(ui));
                section(ui, "Install, update and adopt", |ui| {
                    self.show_manual_install(ui)
                });
                section(ui, "Client patches", |ui| {
                    self.show_client_patches_toggle(ui)
                });
                section(ui, "Debug launches", |ui| self.show_debug_launches(ui));
                section(ui, "Debug logs", |ui| self.show_log_upload(ui));
                section(ui, "Client state", |ui| self.show_client_state(ui));
                section(ui, "Launcher updates", |ui| self.show_update_controls(ui));
                self.show_client_changes(ui);
                section(ui, "Activity log", |ui| self.show_status_log(ui));
            });
    }

    fn show_login_servers(&mut self, ui: &mut egui::Ui) {
        ui.label(theme::muted(
            "One `Name = http://host:8081` per line. Saved into the client's \
             LoginInternal.lua before every launch.",
        ));
        ui.add(
            egui::TextEdit::multiline(&mut self.login_servers_text)
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );
        if ui.button("Save login servers").clicked() {
            self.save_config();
        }
    }

    fn show_manifest_source(&mut self, ui: &mut egui::Ui) {
        // Edits stay pending until Refresh: changing the URL in place would
        // pair the last verified manifest with another host's blobs.
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.manifest_url_text)
                    .desired_width(ui.available_width() - 90.0),
            );
            if ui.button("Refresh").clicked() {
                self.config.manifest_url = self.manifest_url_text.trim().to_owned();
                self.refresh_manifest();
            }
        });
        if self.manifest_url_text.trim() != self.config.manifest_url {
            ui.label(theme::muted("Not in use until you press Refresh."));
        }
        let slot = &self.play.manifest;
        match (&slot.manifest, &slot.error) {
            (Some(m), _) => {
                ui.label(theme::muted(format!(
                    "Signature verified. Schema {}, seed {} ({}), {} patch(es).",
                    m.schema,
                    m.seed.blob,
                    super::human_bytes(m.seed.size),
                    m.patches.len()
                )));
            }
            (None, Some(e)) => {
                ui.colored_label(theme::DANGER, format!("Manifest error: {e}"));
            }
            (None, None) => {
                ui.label(theme::muted("Fetching manifest…"));
            }
        }
    }

    fn show_manual_install(&mut self, ui: &mut egui::Ui) {
        let block = file_action_block(&self.play);
        if let Some(why) = block {
            ui.label(theme::muted(why));
        }
        if should_show_adopt_button(&self.config.install_path) {
            ui.label(theme::muted(
                "This folder has SGW.exe but the launcher has not adopted it. Adopt skips \
                 the seed download and lets patches apply on top. The seed's files are NOT \
                 verified: if they do not match the published seed, patches may misbehave.",
            ));
            if ui
                .add_enabled(
                    block.is_none() && self.play.manifest.manifest.is_some(),
                    egui::Button::new("Adopt existing install"),
                )
                .clicked()
            {
                self.start_adopt();
            }
        }
        if self.installed.seed_adopted {
            ui.colored_label(
                theme::WARN,
                "Seed marked as adopted (unverified). Patches apply on top.",
            );
        }
        if let Some(m) = &self.play.manifest.manifest {
            let missing: Vec<&str> = self
                .installed
                .missing_patches(&m.patches)
                .into_iter()
                .map(|p| p.id.as_str())
                .collect();
            let seed_ok = self.installed.seed_sha256.as_deref() == Some(m.seed.sha256.as_str());
            ui.label(theme::muted(match (seed_ok, missing.is_empty()) {
                (true, true) => "The ledger says everything in the manifest is installed.".into(),
                (false, _) => {
                    "Seed not installed (or a different seed): Install downloads it.".into()
                }
                (true, false) => format!("Not yet applied: {}", missing.join(", ")),
            }));
        }
        let can = block.is_none()
            && self.play.manifest.manifest.is_some()
            && !path_is_empty(&self.config.install_path)
            && self.writable
            && !self.launcher_blocked();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(can, egui::Button::new("Install / Update"))
                .clicked()
            {
                self.start_install();
            }
            let installing = self.play.operation.is_some();
            if ui
                .add_enabled(installing, egui::Button::new("Cancel"))
                .clicked()
            {
                self.cancel_install();
            }
        });
    }

    /// The client-patches opt-out. Saved as soon as it changes, so the
    /// next launch honours it without a separate Save click.
    fn show_client_patches_toggle(&mut self, ui: &mut egui::Ui) {
        let changed = ui
            .checkbox(
                &mut self.config.client_patches.enabled,
                "Load client patches (restores the Black Market window)",
            )
            .on_hover_text(
                "Injects cimmeria-client-patches.dll into SGW.exe on Play. Independent of \
                 diagnostics. Atera debug launches never load it.",
            )
            .changed();
        if changed {
            let state = if self.config.client_patches.enabled {
                "on"
            } else {
                "off"
            };
            match self.config.save(&self.config_path) {
                Ok(_) => self.push_status(format!("Client patches {state} for the next launch.")),
                Err(e) => {
                    self.push_status(format!("Client patches {state}, but saving failed: {e}"))
                }
            }
        }
    }

    fn show_debug_launches(&mut self, ui: &mut egui::Ui) {
        let dir = crate::install_layout::binaries_dir(&self.config.install_path);
        let opts = self.launch_opts.clone();
        let blocked = launch_block(&self.play, &self.inputs());
        if let Some(why) = blocked {
            ui.label(theme::muted(why));
        }
        let allowed = blocked.is_none();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    opts.atera_available() && allowed,
                    egui::Button::new("Launch Atera Debug"),
                )
                .clicked()
            {
                self.worker.dispatch(Command::LaunchAteraDebug {
                    dir: dir.clone(),
                    prep: self.client_prep(),
                });
            }
            // Telemetry-enabled launch needs identity + opt-in + Atera.
            let telemetry_ready = allowed
                && opts.atera_available()
                && self.config.telemetry.opted_in
                && self.identity.is_some();
            if ui
                .add_enabled(
                    telemetry_ready,
                    egui::Button::new("Launch Atera + Telemetry"),
                )
                .clicked()
            {
                if let Some(id) = &self.identity {
                    let telemetry = build_telemetry_config(&self.config, id);
                    let prep = self.client_prep();
                    self.worker
                        .dispatch(Command::LaunchAteraDebugWithTelemetry {
                            install_dir: dir.clone(),
                            telemetry,
                            prep,
                        });
                }
            }
            if ui
                .add_enabled(
                    opts.atera_fix_aslr_bat_present && allowed,
                    egui::Button::new("Fix ASLR"),
                )
                .clicked()
            {
                self.worker
                    .dispatch(Command::LaunchAteraFixAslr(dir.clone()));
            }
        });
        ui.label(theme::muted(
            "Atera debug needs AteraLoader.exe + AtreaGameDebug.bat beside SGW.exe, and ASLR \
             off in SGW.exe (Play switches it off; \"Fix ASLR\" does it by hand). The launcher \
             cannot follow a game the bat starts; it notices the running SGW.exe instead.",
        ));
    }

    fn show_log_upload(&mut self, ui: &mut egui::Ui) {
        let sas = LOG_UPLOAD_SAS_URL;
        let enabled = sas.is_some() && !path_is_empty(&self.config.install_path);
        if ui
            .add_enabled(enabled, egui::Button::new("Upload Debug Logs"))
            .clicked()
        {
            if let Some(sas) = sas {
                self.push_status("Preparing debug logs…".into());
                self.worker.dispatch(Command::UploadLogs {
                    install_dir: self.config.install_path.clone(),
                    sas_url: sas.to_string(),
                    ledger_path: ledger_path(),
                });
            }
        }
        if sas.is_none() {
            ui.label(theme::muted(
                "Log upload disabled: this build has no LAUNCHER_LOG_SAS_URL baked in.",
            ));
        }
        ui.label(theme::muted(
            "Zips Binaries/sgwdebuglog* + Binaries/sessions/** into one file and uploads it. \
             An already-uploaded log set is not sent twice.",
        ));
    }

    /// The two reset buttons backed by [`crate::client_paths`]. The client
    /// consults its cache in `Documents\My Games\Firesky\` before the
    /// launcher-managed PAKs, so a stale cache from a previous server can
    /// win silently; these are the supported recovery path
    /// (docs/architecture/mission-pak-overrides.md).
    fn show_client_state(&mut self, ui: &mut egui::Ui) {
        ui.label(theme::muted(
            "The client writes its cache and per-user settings to \
             Documents\\My Games\\Firesky\\. The cache is read before the launcher-managed \
             PAKs, so stale entries from a previous server can win silently.",
        ));
        let block = file_action_block(&self.play);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(block.is_none(), egui::Button::new("Reset client cache"))
                .on_disabled_hover_text(block.unwrap_or(""))
                .clicked()
            {
                self.worker.dispatch(Command::WipeClientCache);
                self.push_status("Wiping Cache.en-US…".into());
            }
            if ui
                .add_enabled(
                    block.is_none(),
                    theme::danger_button("Reset all client state…"),
                )
                .on_disabled_hover_text(block.unwrap_or(""))
                .clicked()
            {
                self.confirm_wipe_all_open = true;
            }
        });
        ui.label(theme::muted(
            "Reset client cache: wipes Cache.en-US (server-pushed PAK overrides); safe \
             whenever you switch servers. Reset all client state: wipes the whole Firesky \
             folder, including saved settings; use it only if the client crashes at once \
             on launch.",
        ));
    }

    /// Modal confirmation for "Reset all client state…".
    pub(super) fn show_confirm_wipe_all_modal(&mut self, ctx: &egui::Context) {
        if !self.confirm_wipe_all_open {
            return;
        }
        let modal = egui::Modal::new(egui::Id::new("confirm-wipe-all-client-state"));
        let response = modal.show(ctx, |ui| {
            ui.heading("Reset all client state?");
            ui.label(
                "This permanently deletes every file under:\n\n\
                 Documents\\My Games\\Firesky\\\n\n\
                 including per-user settings, keybinds or screenshots the client saved \
                 there. The cache regenerates on the next launch; settings return to \
                 their defaults.\n\n\
                 This is the documented recovery for cache corruption (the client crashes \
                 immediately on launch). For a lighter fix, use \"Reset client cache\", \
                 which wipes only the server-pushed PAK overrides.",
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    self.confirm_wipe_all_open = false;
                }
                if ui.add(theme::danger_button("Delete everything")).clicked() {
                    self.worker.dispatch(Command::WipeAllClientState);
                    self.push_status("Wiping Firesky/ tree…".into());
                    self.confirm_wipe_all_open = false;
                }
            });
        });
        if response.should_close() {
            self.confirm_wipe_all_open = false;
        }
    }

    fn show_status_log(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("status-log")
            .max_height(180.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let start = self.status.len().saturating_sub(100);
                for line in &self.status[start..] {
                    ui.label(RichText::new(line).small().color(theme::MUTED));
                }
            });
    }
}

/// One collapsible group inside Advanced.
fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::CollapsingHeader::new(title)
        .id_salt(("advanced", title))
        .show(ui, body);
}
