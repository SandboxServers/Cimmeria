//! The "Changes to your client" list: every way this launcher makes the
//! client differ from the stock 2009 install. The rows come from
//! [`crate::client_changes::list`]; this file only draws them.

use eframe::egui;

use super::{path_is_empty, should_show_adopt_button, LauncherApp};
use crate::client_changes::{list, ChangeInputs};

impl LauncherApp {
    pub(super) fn show_client_changes(&mut self, ui: &mut egui::Ui) {
        let patches = self
            .manifest
            .as_ref()
            .map(|m| m.patches.as_slice())
            .unwrap_or(&[]);
        let rows = list(&ChangeInputs {
            patches,
            installed: &self.installed,
            client_patches_enabled: self.config.client_patches.enabled,
            telemetry_opted_in: self.config.telemetry.opted_in,
        });
        // Open by default until the launcher manages a client, so a
        // player sees what will change before Install or Adopt.
        let unmanaged = path_is_empty(&self.config.install_path)
            || self.installed.seed_sha256.is_none()
            || should_show_adopt_button(&self.config.install_path);
        egui::CollapsingHeader::new(
            egui::RichText::new(format!("Changes to your client ({})", rows.len())).strong(),
        )
        .id_salt("client-changes")
        .default_open(unmanaged)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Stargate Worlds needs the stock 2009 client. Whether the launcher \
                     downloaded it or you pointed it at your own copy, it changes that \
                     copy as listed here.",
                )
                .small()
                .italics(),
            );
            if self.manifest.is_none() {
                ui.label(
                    egui::RichText::new(
                        "The patch list appears once the manifest has been fetched.",
                    )
                    .small(),
                );
            }
            egui::ScrollArea::vertical()
                .id_salt("client-changes-scroll")
                .max_height(240.0)
                .show(ui, |ui| {
                    let mut group = None;
                    for row in &rows {
                        if group != Some(row.group) {
                            group = Some(row.group);
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(row.group.heading()).underline());
                        }
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(&row.title).strong());
                            ui.label(
                                egui::RichText::new(format!("({})", row.status.label())).small(),
                            );
                            if let Some(id) = &row.patch_id {
                                if *id != row.title {
                                    ui.label(egui::RichText::new(id).small().weak());
                                }
                            }
                        });
                        ui.label(egui::RichText::new(&row.description).small());
                    }
                });
        });
    }
}
