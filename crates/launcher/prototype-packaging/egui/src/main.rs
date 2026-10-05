#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui::{self, Color32, RichText};
use packaging_proof_model::Model;
struct Proof {
    state: Model,
}
impl eframe::App for Proof {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("CIMMERIA").color(Color32::from_rgb(135, 213, 231)));
                ui.label("· egui / Metal packaging proof");
            });
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.selectable_label(!self.state.notes, "Play").clicked() {
                    self.state.apply("home");
                }
                if ui
                    .selectable_label(self.state.notes, "Patch Notes")
                    .clicked()
                {
                    self.state.apply("notes");
                }
                if ui.button("⚙ Settings").clicked() {
                    self.state.apply("settings");
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(440.0)
                .show(ui, |ui| {
                    if self.state.settings {
                        ui.heading("Game settings");
                        ui.label("Local install directory: not configured in this proof");
                        ui.horizontal(|ui| {
                            for (label, action) in [
                                ("Open folder", "folder"),
                                ("Repair game", "repair"),
                                ("Uninstall…", "uninstall"),
                            ] {
                                if ui.button(label).clicked() {
                                    self.state.apply(action);
                                }
                            }
                        });
                        ui.separator();
                    }
                    if self.state.notes {
                        ui.heading("Patch Notes");
                        ui.label("Bundled release snapshot — not proof of installed patches.");
                        ui.label(include_str!("../../notes.txt"));
                    } else {
                        ui.add_space(12.0);
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 120.0),
                            egui::Sense::hover(),
                        );
                        let center = rect.center();
                        ui.painter()
                            .circle_filled(center, 51.0, Color32::from_rgb(21, 49, 63));
                        ui.painter().circle_stroke(
                            center,
                            54.0,
                            egui::Stroke::new(8.0, Color32::from_rgb(72, 120, 139)),
                        );
                        ui.painter().circle_stroke(
                            center,
                            44.0,
                            egui::Stroke::new(2.0, Color32::from_rgb(135, 213, 231)),
                        );
                        ui.heading(RichText::new("Stargate Worlds").size(34.0));
                        ui.label("One game. One launcher. Your next adventure.");
                        ui.add_space(18.0);
                        if ui
                            .add_sized(
                                [ui.available_width(), 48.0],
                                egui::Button::new(
                                    RichText::new("Install").color(Color32::from_rgb(16, 38, 46)),
                                )
                                .fill(Color32::from_rgb(135, 213, 231)),
                            )
                            .clicked()
                        {
                            self.state.apply("install");
                        }
                        ui.label("Simulation only · no game downloads");
                    }
                });
            ui.add_space(12.0);
            ui.separator();
            ui.checkbox(
                &mut self.state.telemetry,
                "Share diagnostic logs · optional",
            );
            ui.label("In-memory preference. No telemetry is transmitted.");
            ui.add_space(10.0);
            ui.label(&self.state.status);
            ui.small("PACKAGING PROTOTYPE · no installation, Wine, or runtime downloads");
        });
    }
}
fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Stargate Worlds — egui proof",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([620.0, 700.0])
                .with_min_inner_size([480.0, 560.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(Proof {
                state: Model::initial(),
            }))
        }),
    )
}
