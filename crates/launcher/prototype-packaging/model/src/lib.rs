//! Throwaway packaging proof: no network, installation or persistence.
use serde::Serialize;
#[derive(Default, Serialize)]
pub struct Model {
    pub telemetry: bool,
    pub settings: bool,
    pub notes: bool,
    pub status: String,
}
impl Model {
    pub fn initial() -> Self {
        Self {
            status: "Ready to open. No downloads required by this prototype.".into(),
            ..Self::default()
        }
    }
    pub fn apply(&mut self, action: &str) {
        match action {
            "telemetry" => self.telemetry = !self.telemetry,
            "settings" => self.settings = !self.settings,
            "notes" => self.notes = true,
            "home" => self.notes = false,
            "install" => self.status = "Simulation: Install would start prerequisite checks and game downloads. Nothing was changed.".into(),
            "repair" | "uninstall" | "folder" => self.status = "Prototype only: no game files exist and no filesystem action was taken.".into(),
            _ => self.status = "Unknown prototype action.".into(),
        }
    }
}
