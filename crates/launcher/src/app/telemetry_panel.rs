//! The telemetry opt-in: a one-time prompt and the checkbox.
//!
//! Telemetry is off until the player turns it on. The prompt asks once
//! (either answer is remembered in `telemetry.prompt_answered`); the
//! checkbox beside the launch buttons changes it any time after.

use eframe::egui;

use super::LauncherApp;

const WHAT_IT_SENDS: &str = "While the game runs, the launcher uploads the client's log \
    files, and an observer DLL in the game reports what the game is doing (loading, frame \
    time, network and UI events, crashes), with this install's random id, to the Cimmeria \
    server, so crashes and bugs can be traced. Nothing is sent while it is off.";

impl LauncherApp {
    /// The first-run question. Hidden once answered either way.
    pub(super) fn show_telemetry_prompt(&mut self, ui: &mut egui::Ui) {
        if self.config.telemetry.prompt_answered {
            return;
        }
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new("Help us fix bugs? (optional)").strong());
            ui.label(format!("Telemetry is off. {WHAT_IT_SENDS}"));
            ui.horizontal(|ui| {
                if ui.button("Turn on telemetry").clicked() {
                    self.answer_telemetry_prompt(true);
                }
                if ui.button("No thanks").clicked() {
                    self.answer_telemetry_prompt(false);
                }
            });
        });
        ui.separator();
    }

    /// The checkbox beside the launch buttons.
    pub(super) fn show_telemetry_toggle(&mut self, ui: &mut egui::Ui) {
        let mut opted_in = self.config.telemetry.opted_in;
        let changed = ui
            .checkbox(&mut opted_in, "Send telemetry (opt-in)")
            .on_hover_text(WHAT_IT_SENDS)
            .changed();
        if changed {
            self.set_telemetry(opted_in);
        }
    }

    fn answer_telemetry_prompt(&mut self, opted_in: bool) {
        self.config.telemetry.prompt_answered = true;
        self.set_telemetry(opted_in);
    }

    /// Record the choice and save it at once, so the next launch honours
    /// it without a separate Save click.
    fn set_telemetry(&mut self, opted_in: bool) {
        self.config.telemetry.opted_in = opted_in;
        self.config.telemetry.prompt_answered = true;
        let state = if opted_in {
            "on from the next launch"
        } else {
            "off; nothing will be sent"
        };
        match self.config.save(&self.config_path) {
            Ok(_) => self.push_status(format!("Telemetry {state}.")),
            Err(e) => self.push_status(format!("Telemetry {state}, but saving failed: {e}")),
        }
    }
}
