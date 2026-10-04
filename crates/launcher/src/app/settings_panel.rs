//! The gear's settings panel: the install folder (change, open in
//! Explorer), Repair and Uninstall, and the Advanced tools (#1153).
//!
//! Changing the folder only changes where the launcher looks. It never
//! moves or deletes the old installation, and it is off while an install
//! runs or the game is running. The new folder is checked first and the
//! player confirms what was found there.

use std::path::{Path, PathBuf};

use eframe::egui::{self, RichText};

use super::play_state::file_action_block;
use super::{theme, LauncherApp};
use crate::worker::Command;

/// What a candidate install folder holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FolderFinding {
    /// Missing or empty: Install puts the game there.
    Empty,
    /// A game this launcher manages (it has `launcher-installed.json`).
    ManagedGame,
    /// `SGW.exe` without the launcher's ledger: it can be adopted.
    UnmanagedGame,
    /// Other files and no game: the game installs alongside them.
    OtherFiles,
}

impl FolderFinding {
    fn describe(&self) -> &'static str {
        match self {
            FolderFinding::Empty => "Empty or not created yet. Install downloads the game here.",
            FolderFinding::ManagedGame => {
                "Holds a Stargate Worlds installation this launcher manages."
            }
            FolderFinding::UnmanagedGame => {
                "Holds SGW.exe that this launcher has not adopted. The Play tab offers to \
                 adopt it."
            }
            FolderFinding::OtherFiles => {
                "Holds other files and no game. The game installs alongside them; nothing \
                 there is removed."
            }
        }
    }
}

/// A checked folder change, waiting for the player to confirm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PendingFolder {
    pub path: PathBuf,
    pub finding: FolderFinding,
}

/// Check a typed folder before it becomes the install folder. Nothing on
/// disk changes here.
pub(super) fn check_new_folder(text: &str, current: &Path) -> Result<PendingFolder, String> {
    let text = text.trim().trim_matches('"');
    if text.is_empty() {
        return Err("Enter a folder.".into());
    }
    let path = PathBuf::from(text);
    if !is_absolute(&path) {
        return Err("Enter a full path, such as C:\\Games\\Stargate Worlds.".into());
    }
    if path.parent().is_none_or(|p| p.as_os_str().is_empty()) {
        return Err("Choose a folder, not the root of a drive.".into());
    }
    if same_path(&path, current) {
        return Err("That is already the install folder.".into());
    }
    if path.is_file() {
        return Err("That is a file, not a folder.".into());
    }
    let finding = if crate::install_layout::sgw_exe(&path).is_file() {
        if crate::state::InstalledState::path(&path).exists() {
            FolderFinding::ManagedGame
        } else {
            FolderFinding::UnmanagedGame
        }
    } else if std::fs::read_dir(&path).is_ok_and(|mut d| d.next().is_some()) {
        FolderFinding::OtherFiles
    } else {
        FolderFinding::Empty
    };
    Ok(PendingFolder { path, finding })
}

/// A drive-qualified absolute path on Windows; `/`-rooted elsewhere, so
/// the tests read the same on CI's Linux runners.
fn is_absolute(p: &Path) -> bool {
    if cfg!(windows) {
        p.is_absolute()
    } else {
        p.is_absolute() || p.to_string_lossy().chars().nth(1) == Some(':')
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    n(a) == n(b)
}

impl LauncherApp {
    pub(super) fn show_settings(&mut self, ui: &mut egui::Ui) {
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Game settings").size(18.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        self.settings_open = false;
                    }
                });
            });
            ui.add_space(4.0);
            self.show_install_folder(ui);
            ui.add_space(10.0);
            self.show_maintenance(ui);
            ui.add_space(6.0);
            self.show_advanced(ui);
        });
        ui.add_space(12.0);
    }

    fn show_install_folder(&mut self, ui: &mut egui::Ui) {
        ui.label(theme::muted("Install folder"));
        let current = self.config.install_path.display().to_string();
        ui.label(
            RichText::new(if current.is_empty() {
                "(not set)"
            } else {
                &current
            })
            .monospace()
            .color(theme::MUTED),
        );
        let block = file_action_block(&self.play);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!current.is_empty(), egui::Button::new("Open in Explorer ↗"))
                .clicked()
            {
                self.settings_notice = Some((false, format!("Opening {current}…")));
                self.worker
                    .dispatch(Command::OpenInExplorer(self.config.install_path.clone()));
            }
            if self.folder_edit.is_none()
                && ui
                    .add_enabled(block.is_none(), egui::Button::new("Change folder…"))
                    .on_disabled_hover_text(block.unwrap_or(""))
                    .clicked()
            {
                self.folder_edit = Some(current.clone());
                self.pending_folder = None;
                self.settings_notice = None;
            }
        });
        if let Some(text) = self.folder_edit.as_mut() {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(text).desired_width(360.0));
            });
            let text = text.clone();
            ui.horizontal(|ui| {
                if ui.button("Check folder").clicked() {
                    match check_new_folder(&text, &self.config.install_path) {
                        Ok(p) => {
                            self.pending_folder = Some(p);
                            self.settings_notice = None;
                        }
                        Err(e) => {
                            self.pending_folder = None;
                            self.settings_notice = Some((true, e));
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    self.folder_edit = None;
                    self.pending_folder = None;
                }
            });
        }
        if let Some(pending) = self.pending_folder.clone() {
            ui.label(RichText::new(pending.path.display().to_string()).monospace());
            ui.label(theme::muted(pending.finding.describe()));
            if !current.is_empty() {
                ui.label(theme::muted(format!(
                    "The installation at {current} stays where it is; nothing is moved or deleted."
                )));
            }
            if ui
                .add_enabled(block.is_none(), egui::Button::new("Use this folder"))
                .on_disabled_hover_text(block.unwrap_or(""))
                .clicked()
            {
                self.change_install_folder(pending.path);
            }
        }
        if let Some((error, text)) = &self.settings_notice {
            let colour = if *error { theme::DANGER } else { theme::ACCENT };
            ui.colored_label(colour, text);
        }
    }

    /// Point the launcher at `path` and save. The old folder is untouched.
    fn change_install_folder(&mut self, path: PathBuf) {
        // Re-check now, not only when the button was drawn: a game may
        // have started from the old folder since the last refresh.
        self.play.probed_pids = crate::game_process::running_game_pids(&self.config.install_path);
        if let Some(why) = file_action_block(&self.play) {
            self.settings_notice = Some((true, format!("Not changed: {why}")));
            return;
        }
        let old = std::mem::replace(&mut self.config.install_path, path);
        self.install_path_text = self.config.install_path.to_string_lossy().into_owned();
        self.folder_edit = None;
        self.pending_folder = None;
        match self.config.save(&self.config_path) {
            Ok(()) => {
                let msg = format!(
                    "Install folder changed. {} was not moved or deleted.",
                    old.display()
                );
                self.push_status(msg.clone());
                self.settings_notice = Some((false, msg));
            }
            Err(e) => {
                let msg = format!(
                    "Install folder changed for this window, but saving failed: {e}. It \
                     reverts when the launcher restarts."
                );
                self.push_status(msg.clone());
                self.settings_notice = Some((true, msg));
            }
        }
        self.refresh_install_state();
    }

    fn show_maintenance(&mut self, ui: &mut egui::Ui) {
        ui.label(theme::muted(
            "Repair will replace missing or damaged game files and keep your settings. \
             Uninstall will remove the game files only, after you confirm.",
        ));
        ui.horizontal(|ui| {
            const LATER: &str = "Arrives in a later launcher update (#1153).";
            ui.add_enabled(false, egui::Button::new("Repair game"))
                .on_disabled_hover_text(LATER);
            ui.add_enabled(false, theme::danger_button("Uninstall…"))
                .on_disabled_hover_text(LATER);
        });
        ui.label(theme::muted(
            "Repair and Uninstall arrive in a later launcher update. Until then, Install in \
             Advanced re-applies anything the launcher has not recorded as installed.",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abs(dir: &Path) -> String {
        dir.to_string_lossy().into_owned()
    }

    #[test]
    fn relative_roots_and_the_current_folder_are_refused() {
        let cur = Path::new("C:\\Games\\SGW");
        assert!(check_new_folder("", cur).is_err());
        assert!(check_new_folder("Games\\SGW", cur).is_err());
        assert!(check_new_folder("C:\\", cur).is_err());
        assert!(check_new_folder("c:/games/sgw/", cur)
            .unwrap_err()
            .contains("already"));
    }

    #[test]
    fn a_missing_folder_is_empty_and_is_not_created() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("New Game");
        let p = check_new_folder(&abs(&target), Path::new("C:\\x")).unwrap();
        assert_eq!(p.finding, FolderFinding::Empty);
        assert!(!target.exists(), "checking must not create the folder");
    }

    #[test]
    fn the_finding_names_what_is_in_the_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let cur = Path::new("C:\\elsewhere");
        let other = tmp.path().join("other");
        std::fs::create_dir(&other).unwrap();
        std::fs::write(other.join("notes.txt"), b"x").unwrap();
        assert_eq!(
            check_new_folder(&abs(&other), cur).unwrap().finding,
            FolderFinding::OtherFiles
        );

        let game = tmp.path().join("game");
        std::fs::create_dir(&game).unwrap();
        std::fs::write(game.join("SGW.exe"), b"").unwrap();
        assert_eq!(
            check_new_folder(&abs(&game), cur).unwrap().finding,
            FolderFinding::UnmanagedGame
        );
        std::fs::write(crate::state::InstalledState::path(&game), b"{}").unwrap();
        assert_eq!(
            check_new_folder(&abs(&game), cur).unwrap().finding,
            FolderFinding::ManagedGame
        );
    }

    #[test]
    fn a_file_is_not_a_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("SGW.exe");
        std::fs::write(&f, b"").unwrap();
        assert!(check_new_folder(&abs(&f), Path::new("C:\\x")).is_err());
    }
}
