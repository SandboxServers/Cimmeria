//! The window frame: layout A (split) of the approved launcher design.
//!
//! A gate panel on the left; on the right the update banner, the heading
//! for the current state, the Play / Patch Notes tabs with the settings
//! gear, the settings panel when open, and the selected tab; the
//! diagnostics opt-in is a footer pinned below all of them. Below
//! [`NARROW_WIDTH`] the gate panel is left out, as in the prototype.

use eframe::egui::{self, Margin, RichText};

use super::play_state::{GameActivity, Primary};
use super::{gate_art, theme, LauncherApp, Tab, REFRESH_EVERY};

/// Narrower than this, the main column takes the whole window.
const NARROW_WIDTH: f32 = 760.0;

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.drain_events(&ctx);
        if self.play.operation.is_some() || self.play.activity() == GameActivity::Launching {
            // Keep animating progress and spinners between events.
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        } else {
            // Wake for the next refresh so a game closed outside the
            // launcher returns the surface to Play without a mouse move.
            ctx.request_repaint_after(REFRESH_EVERY);
        }
        if self.last_refresh.elapsed() > REFRESH_EVERY {
            self.refresh_install_state();
        }

        let wide = ui.available_width() >= NARROW_WIDTH;
        if wide {
            let width = (ui.available_width() * 0.34).clamp(260.0, 360.0);
            egui::Panel::left("gate-panel")
                .exact_size(width)
                .resizable(false)
                .show_separator_line(true)
                .frame(egui::Frame::new().fill(theme::ASIDE_BOTTOM))
                .show(ui, |ui| self.show_aside(ui));
        }
        let side_margin = if wide { 44 } else { 24 };
        // The diagnostics opt-in is pinned below the main column, so it
        // stays on screen on both tabs and with Settings open.
        egui::Panel::bottom("telemetry-footer")
            .resizable(false)
            .show_separator_line(true)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(Margin::symmetric(side_margin, 12)),
            )
            .show(ui, |ui| self.show_telemetry_control(ui));
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(Margin::symmetric(side_margin, 28)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("main-column")
                    .auto_shrink(false)
                    .show(ui, |ui| self.show_main_column(ui));
            });
        self.show_confirm_wipe_all_modal(&ctx);
    }
}

impl LauncherApp {
    fn show_aside(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        gate_art::vertical_gradient(ui.painter(), rect, theme::ASIDE_TOP, theme::ASIDE_BOTTOM);
        egui::Frame::new()
            .inner_margin(Margin::symmetric(30, 34))
            .show(ui, |ui| {
                ui.label(theme::eyebrow("Cimmeria · for Windows"));
                ui.add_space(36.0);
                ui.vertical_centered(|ui| {
                    gate_art::gate(ui, (ui.available_width() * 0.62).min(180.0));
                    ui.add_space(18.0);
                    ui.label(
                        RichText::new(theme::spaced_caps("A world beyond"))
                            .size(10.0)
                            .color(egui::Color32::from_rgb(0x9f, 0xc0, 0xc9)),
                    );
                });
                ui.add_space(40.0);
                ui.label(
                    RichText::new("Stargate Worlds.\nInstall once. Step through the gate.")
                        .size(12.5)
                        .color(egui::Color32::from_rgb(0xa4, 0xba, 0xc6)),
                );
            });
    }

    fn show_main_column(&mut self, ui: &mut egui::Ui) {
        self.show_update_banner(ui);
        let pill = egui::Button::new(RichText::new("STARGATE WORLDS").size(11.0))
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::new(1.0, theme::LINE))
            .corner_radius(30)
            .sense(egui::Sense::hover());
        ui.add(pill);
        ui.add_space(6.0);
        let (heading, sub) = self.heading();
        ui.label(RichText::new(heading).size(38.0).strong());
        ui.label(RichText::new(sub).color(theme::MUTED).size(14.0));
        ui.add_space(12.0);
        self.show_tabs(ui);
        ui.separator();
        ui.add_space(10.0);
        if self.settings_open {
            self.show_settings(ui);
        }
        match self.tab {
            Tab::Play => self.show_play_tab(ui),
            Tab::Notes => self.show_patch_notes(ui),
        }
        ui.add_space(18.0);
    }

    fn show_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (tab, label) in [(Tab::Play, "Play"), (Tab::Notes, "Patch Notes")] {
                let selected = self.tab == tab;
                let text = RichText::new(label).size(14.0).color(if selected {
                    theme::ACCENT
                } else {
                    theme::MUTED
                });
                let button = egui::Button::new(text)
                    .fill(if selected {
                        theme::ACCENT.gamma_multiply(0.08)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .stroke(egui::Stroke::NONE)
                    .corner_radius(8);
                if ui.add(button).clicked() {
                    self.tab = tab;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let gear =
                    egui::Button::new(RichText::new("⚙").size(18.0).color(if self.settings_open {
                        theme::ACCENT
                    } else {
                        theme::MUTED
                    }))
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(egui::Stroke::NONE);
                if ui.add(gear).on_hover_text("Game settings").clicked() {
                    self.settings_open = !self.settings_open;
                    if !self.settings_open {
                        self.folder_edit = None;
                        self.pending_folder = None;
                    }
                }
            });
        });
    }

    /// The heading and subtitle for the state the surface is in.
    fn heading(&self) -> (&'static str, &'static str) {
        if self.tab == Tab::Notes {
            return (
                "Your next adventure.",
                "What the current game content changes.",
            );
        }
        match super::play_state::primary_action(&self.play, &self.inputs()) {
            Primary::Install | Primary::ChooseFolder | Primary::Adopt => (
                "Your next adventure,\nwithout the setup.",
                "The original Stargate Worlds, on Cimmeria.",
            ),
            Primary::Installing | Primary::Busy(_) => (
                "We'll take it\nfrom here.",
                "Everything your game needs, in one place.",
            ),
            Primary::Play | Primary::Update => {
                ("The gate is ready.", "Continue your journey on Cimmeria.")
            }
            Primary::Launching | Primary::Running => {
                ("See you on\nthe other side.", "Your adventure is running.")
            }
            Primary::Unavailable(_) => (
                "Let's get you\nback on track.",
                "A fixable setup issue. No need to start over.",
            ),
        }
    }
}
