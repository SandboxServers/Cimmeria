//! The Play tab: one installation surface that turns into Play.
//!
//! A status card says where things stand; under it, the primary action
//! from [`primary_action`] and, where it helps, one secondary action.
//! Download, extract and apply are progress phases on this same card,
//! not separate screens. Every click changes the card in the same frame
//! (the reducer's `click_*`), before the worker answers.

use eframe::egui::{self, RichText};

use super::play_state::{offers_play_anyway, primary_action, InstallStatus, Primary};
use super::status_lines::download_progress_line;
use super::{human_bytes, theme, LauncherApp};
use crate::install::Progress;

impl LauncherApp {
    pub(super) fn show_play_tab(&mut self, ui: &mut egui::Ui) {
        let inputs = self.inputs();
        let primary = primary_action(&self.play, &inputs);
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.show_status_card(ui, &primary, &inputs.status);
        });
        ui.add_space(16.0);
        ui.horizontal_wrapped(|ui| {
            self.show_primary_button(ui, &primary);
            if offers_play_anyway(&primary, &inputs)
                && ui
                    .add(theme::secondary_button("Play without updating"))
                    .clicked()
            {
                self.start_play();
            }
            let label = if self.show_details {
                "Hide details"
            } else {
                "View details"
            };
            if ui.add(theme::secondary_button(label)).clicked() {
                self.show_details = !self.show_details;
            }
        });
        if let Some(n) = &self.play.notice {
            ui.add_space(6.0);
            let colour = if n.error {
                theme::DANGER
            } else {
                theme::ACCENT
            };
            ui.colored_label(colour, &n.text);
        }
        if self.show_details {
            ui.add_space(6.0);
            self.show_details(ui);
        }
        if primary == Primary::Install {
            ui.add_space(10.0);
            ui.label(theme::muted(
                "A server account is needed to sign in inside the game.",
            ));
        }
    }

    fn show_status_card(&self, ui: &mut egui::Ui, primary: &Primary, status: &InstallStatus) {
        let (title, colour, body): (String, egui::Color32, String) = match primary {
            Primary::ChooseFolder => (
                "Choose where to install".into(),
                theme::INK,
                "Pick a folder in Settings. The launcher downloads the game there.".into(),
            ),
            Primary::Install => {
                let size = self
                    .play
                    .manifest
                    .manifest
                    .as_ref()
                    .map(|m| human_bytes(m.seed.size))
                    .unwrap_or_default();
                (
                    "One setup. Then just Play.".into(),
                    theme::INK,
                    format!(
                        "The launcher downloads the original game ({size}), applies \
                         Cimmeria's patches and sets up the login servers, in \
                         {}. You can cancel and pick up later.",
                        self.config.install_path.display()
                    ),
                )
            }
            Primary::Adopt => (
                "Stargate Worlds found in this folder".into(),
                theme::INK,
                "This folder already has SGW.exe. Use it as it is: the launcher skips \
                 the download and applies Cimmeria's patches on top. Its existing files \
                 are not verified."
                    .into(),
            ),
            Primary::Update => {
                let (seed, patches) = match status {
                    InstallStatus::NeedsUpdate { seed, patches } => (*seed, *patches),
                    _ => (false, 0),
                };
                let what = match (seed, patches) {
                    (true, _) => "A new base game download".to_string(),
                    (false, 1) => "1 patch".to_string(),
                    (false, n) => format!("{n} patches"),
                };
                (
                    "An update is ready".into(),
                    theme::WARN,
                    format!("{what} to apply. Your settings and characters are kept."),
                )
            }
            Primary::Installing => ("Installing".into(), theme::INK, String::new()),
            Primary::Busy(what) => ((*what).into(), theme::INK, String::new()),
            Primary::Play => (
                "● Ready to play".into(),
                theme::GOOD,
                format!(
                    "Game files are installed. Client patches {}.",
                    if self.config.client_patches.enabled {
                        "on"
                    } else {
                        "off"
                    }
                ),
            ),
            Primary::Launching => (
                "Starting Stargate Worlds…".into(),
                theme::ACCENT,
                "Preparing the client and loading Cimmeria's patches.".into(),
            ),
            Primary::Running => (
                "● Game running".into(),
                theme::GOOD,
                "Sign in inside Stargate Worlds. The launcher returns to Play when the game \
                 closes; closing the launcher does not close the game."
                    .into(),
            ),
            Primary::Unavailable(why) => (
                "One more step before you play".into(),
                theme::WARN,
                why.clone(),
            ),
        };
        ui.label(RichText::new(title).size(15.0).color(colour).strong());
        if !body.is_empty() {
            ui.label(theme::muted(body));
        }
        match primary {
            Primary::Installing => self.show_progress(ui),
            Primary::Busy(_) | Primary::Launching => {
                ui.spinner();
            }
            _ => {}
        }
    }

    fn show_progress(&self, ui: &mut egui::Ui) {
        let Some(p) = &self.play.progress else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(theme::muted("Starting…"));
            });
            return;
        };
        let (phase, line, fraction) = match p {
            Progress::Downloading {
                label,
                downloaded,
                total,
            } => (
                "Downloading",
                download_progress_line(label, *downloaded, *total),
                ratio(*downloaded as f64, *total as f64),
            ),
            Progress::Extracting {
                label,
                current,
                total,
                filename,
            } => (
                "Installing files",
                format!("{label}: {current} / {total} — {filename}"),
                ratio(*current as f64, *total as f64),
            ),
        };
        ui.label(RichText::new(phase).strong());
        ui.add(
            egui::ProgressBar::new(fraction)
                .show_percentage()
                .fill(theme::ACCENT),
        );
        ui.label(theme::muted(line));
    }

    fn show_primary_button(&mut self, ui: &mut egui::Ui, primary: &Primary) {
        match primary {
            Primary::ChooseFolder => {
                if ui
                    .add(theme::primary_button("Choose install folder"))
                    .clicked()
                {
                    self.settings_open = true;
                    self.folder_edit = Some(String::new());
                }
            }
            Primary::Install => {
                if ui
                    .add(theme::primary_button("Install Stargate Worlds"))
                    .clicked()
                {
                    self.start_install();
                }
            }
            Primary::Update => {
                if ui.add(theme::primary_button("Update")).clicked() {
                    self.start_install();
                }
            }
            Primary::Adopt => {
                if ui
                    .add(theme::primary_button("Use this installation"))
                    .clicked()
                {
                    self.start_adopt();
                }
            }
            Primary::Play => {
                if ui.add(theme::primary_button("Play")).clicked() {
                    self.start_play();
                }
            }
            Primary::Installing => {
                let installing =
                    self.play.operation == Some(super::play_state::Operation::Installing);
                if ui
                    .add_enabled(installing, theme::secondary_button("Cancel"))
                    .clicked()
                {
                    self.cancel_install();
                }
            }
            Primary::Busy(_) => {
                ui.add_enabled(false, theme::primary_button("Working…"));
            }
            Primary::Launching => {
                ui.add_enabled(false, theme::primary_button("Starting…"));
            }
            Primary::Running => {
                ui.add_enabled(false, theme::primary_button("Playing"));
            }
            Primary::Unavailable(_) => {
                let manifest_failed =
                    self.play.manifest.manifest.is_none() && self.play.manifest.error.is_some();
                if manifest_failed {
                    if ui.add(theme::primary_button("Retry")).clicked() {
                        self.refresh_manifest();
                    }
                } else if ui.add(theme::primary_button("Open settings")).clicked() {
                    self.settings_open = true;
                }
            }
        }
    }

    fn show_details(&self, ui: &mut egui::Ui) {
        let content = match (&self.play.manifest.manifest, &self.play.manifest.error) {
            (Some(m), _) => format!("signed manifest, {} patch(es)", m.patches.len()),
            (None, Some(_)) => "manifest unavailable".into(),
            (None, None) => "checking…".into(),
        };
        let lines = [
            format!("Install folder: {}", self.config.install_path.display()),
            format!(
                "Game: {}",
                if self.launch_opts.sgw_present {
                    "SGW.exe found"
                } else {
                    "SGW.exe not found"
                }
            ),
            format!(
                "Installed by: {}",
                if self.installed.seed_adopted {
                    "adopted existing copy (not verified)"
                } else if self.installed.seed_sha256.is_some() {
                    "this launcher"
                } else {
                    "not installed yet"
                }
            ),
            format!("Content: {content}"),
            format!(
                "Client patches: {}",
                if self.config.client_patches.enabled {
                    "on"
                } else {
                    "off"
                }
            ),
            format!(
                "Diagnostics: {}",
                if self.config.telemetry.opted_in {
                    "on"
                } else {
                    "off"
                }
            ),
        ];
        for l in lines {
            ui.label(theme::muted(l));
        }
    }
}

fn ratio(done: f64, total: f64) -> f32 {
    if total > 0.0 {
        (done / total).min(1.0) as f32
    } else {
        0.0
    }
}
