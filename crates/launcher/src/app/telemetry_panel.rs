//! The diagnostics opt-in: one control in the footer of every view
//! (Play, Patch Notes, and with Settings open), #1153.
//!
//! Off until the player turns it on, saved the moment it changes, and
//! never turned on by installing or playing. A launch reads the choice
//! once, so a change while the game runs applies to the next launch; the
//! caption says so, and tells the running session's state apart from the
//! saved preference.

use std::path::Path;

use eframe::egui::{self, RichText};

use super::play_state::Lifecycle;
use super::{theme, LauncherApp};
use crate::config::{ConfigError, LauncherConfig};

const WHAT_IT_SENDS: &str = "While the game runs, the launcher uploads the client's log \
    files, and a small telemetry module loaded into the game records in-game events \
    (the game messages it receives, interface errors, the module's own status). Both go, \
    with this install's random id, to the Cimmeria server, so crashes and bugs can be \
    traced. Nothing is sent, and the module is not loaded, while this is off.";

/// What the game that is running now was launched with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Session {
    NoGame,
    /// Launched by this window with diagnostics on or off.
    Known(bool),
    /// Running, but not launched by this window (or no longer followed).
    Unknown,
}

/// The caption under the checkbox. `pref` is the saved choice.
pub(super) fn caption(pref: bool, session: Session) -> String {
    match session {
        Session::NoGame => "Your choice applies when you next launch the game.".into(),
        Session::Known(s) if s == pref => format!(
            "This game session: diagnostics {}. A change applies on your next game launch.",
            if s { "on" } else { "off" }
        ),
        Session::Known(true) => "Off from your next game launch. The game running now keeps \
                                  sending diagnostics until it closes."
            .into(),
        Session::Known(false) => "On from your next game launch. Nothing is sent for the game \
                                   running now."
            .into(),
        Session::Unknown => {
            "The game is running. A change applies on your next game launch.".into()
        }
    }
}

impl LauncherApp {
    fn telemetry_session(&self) -> Session {
        match self.play.game {
            Lifecycle::Launching { telemetry } | Lifecycle::Running { telemetry, .. } => {
                Session::Known(telemetry)
            }
            Lifecycle::Idle if !self.play.probed_pids.is_empty() => Session::Unknown,
            Lifecycle::Idle => Session::NoGame,
        }
    }

    /// The footer control. Drawn once per frame by the shell, below
    /// whichever tab and settings view is showing.
    pub(super) fn show_telemetry_control(&mut self, ui: &mut egui::Ui) {
        let mut opted_in = self.config.telemetry.opted_in;
        let state = if opted_in { "On" } else { "Off" };
        let changed = ui
            .checkbox(
                &mut opted_in,
                RichText::new(format!("Share diagnostic logs · {state}")).strong(),
            )
            .on_hover_text(WHAT_IT_SENDS)
            .changed();
        if changed {
            self.set_telemetry(opted_in);
        }
        ui.indent("telemetry-caption", |ui| {
            ui.label(theme::muted(
                "Optional. Helps diagnose crashes and game issues. Change this any time.",
            ));
            ui.label(theme::muted(caption(
                self.config.telemetry.opted_in,
                self.telemetry_session(),
            )));
            if let Some(e) = &self.telemetry_save_error {
                ui.colored_label(theme::DANGER, e);
            }
            egui::CollapsingHeader::new(theme::muted("What is sent?"))
                .id_salt("telemetry-what")
                .show(ui, |ui| {
                    ui.label(theme::muted(WHAT_IT_SENDS));
                });
        });
    }

    /// Record the choice and save it at once.
    fn set_telemetry(&mut self, opted_in: bool) {
        let saved = record_choice(&mut self.config, &self.config_path, opted_in);
        let state = if opted_in {
            "on from the next launch"
        } else {
            "off from the next launch"
        };
        match saved {
            Ok(()) => {
                self.telemetry_save_error = None;
                self.push_status(format!("Diagnostics {state}."));
            }
            Err(e) => {
                self.telemetry_save_error = Some(format!(
                    "Could not save this choice ({e}). It holds only while this launcher \
                     window stays open."
                ));
                self.push_status(format!("Diagnostics {state}, but saving failed: {e}"));
            }
        }
    }
}

/// Set the opt-in in `config` and write the file. On a failed write the
/// choice still holds in memory, for launches from this window.
pub(super) fn record_choice(
    config: &mut LauncherConfig,
    path: &Path,
    opted_in: bool,
) -> Result<(), ConfigError> {
    config.telemetry.opted_in = opted_in;
    config.telemetry.prompt_answered = true;
    config.save(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The choice survives a restart: written at once, read back as is.
    #[test]
    fn a_choice_is_saved_at_once_and_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("launcher-config.json");
        let mut cfg = LauncherConfig::default();
        assert!(!cfg.telemetry.opted_in, "default off");
        for want in [true, false, true] {
            record_choice(&mut cfg, &path, want).unwrap();
            let back = LauncherConfig::load(&path).unwrap();
            assert_eq!(back.telemetry.opted_in, want);
            assert!(back.telemetry.prompt_answered);
        }
    }

    // A failed save is reported, and the choice still applies in memory.
    #[test]
    fn a_failed_save_is_an_error_but_the_choice_holds() {
        let dir = tempfile::tempdir().unwrap();
        // A folder where the file should be: the write's final rename fails.
        let path = dir.path().join("launcher-config.json");
        std::fs::create_dir(&path).unwrap();
        let mut cfg = LauncherConfig::default();
        assert!(record_choice(&mut cfg, &path, true).is_err());
        assert!(cfg.telemetry.opted_in);
    }

    #[test]
    fn with_no_game_a_change_applies_next_launch() {
        for pref in [false, true] {
            assert!(caption(pref, Session::NoGame).contains("next launch"));
        }
    }

    // A change during play must not read as if it stopped (or started)
    // the running session: the session keeps what it launched with.
    #[test]
    fn a_change_during_play_separates_the_session_from_the_preference() {
        let off_now = caption(false, Session::Known(true));
        assert!(off_now.contains("keeps sending"), "{off_now}");
        assert!(off_now.contains("next game launch"), "{off_now}");
        let on_now = caption(true, Session::Known(false));
        assert!(on_now.contains("Nothing is sent for the game"), "{on_now}");
        let same = caption(true, Session::Known(true));
        assert!(same.contains("This game session: diagnostics on"), "{same}");
    }

    #[test]
    fn an_unfollowed_game_says_only_what_is_known() {
        let c = caption(true, Session::Unknown);
        assert!(
            c.contains("next game launch") && !c.contains("session:"),
            "{c}"
        );
    }
}
