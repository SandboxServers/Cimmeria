//! The Install/Play surface's state, kept apart from egui so it is
//! testable.
//!
//! [`PlayState`] is the reducer: worker [`Event`]s and the player's
//! clicks move it between idle, installing, launching and running.
//! [`primary_action`] reads it, with the install facts in [`Inputs`], to
//! decide the one big button; [`file_action_block`] is the guard every
//! file-changing control asks before it is enabled. The worker repeats
//! the same guard (`worker::activity`), so a stale frame cannot start a
//! conflicting job.

use crate::install::Progress;
use crate::manifest::Manifest;
use crate::state::InstalledState;
use crate::worker::{Busy, Event};

/// A long-running file job the launcher started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Operation {
    /// Install clicked; waiting for the worker to accept it.
    StartingInstall,
    Installing,
    Adopting,
}

/// The game, as far as the launcher can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lifecycle {
    Idle,
    /// Play clicked; waiting for `Launched` or an error.
    Launching {
        telemetry: bool,
    },
    /// The launcher started this game and follows its exit.
    Running {
        pid: u32,
        telemetry: bool,
    },
}

/// What the surface shows about the game: the lifecycle, or a game the
/// process probe found that the launcher is not following.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GameActivity {
    Idle,
    Launching,
    Running { telemetry: Option<bool> },
}

/// The outcome line under the install card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Notice {
    pub error: bool,
    pub text: String,
}

impl Notice {
    fn info(text: impl Into<String>) -> Self {
        Self {
            error: false,
            text: text.into(),
        }
    }
    fn error(text: impl Into<String>) -> Self {
        Self {
            error: true,
            text: text.into(),
        }
    }
}

/// The manifest the launcher trusts: only one whose signature verified
/// (the worker never sends another) and that came from the URL in use.
#[derive(Debug, Default)]
pub(super) struct ManifestSlot {
    pub manifest: Option<Manifest>,
    /// The URL `manifest` was fetched from.
    pub source: Option<String>,
    pub error: Option<String>,
    pub fetching: bool,
}

impl ManifestSlot {
    /// A fetch for `url` is starting. A manifest from another URL is
    /// dropped at once: its blobs would be fetched relative to the new
    /// URL, and an install must never mix the two.
    /// The manifest, only if it came from `url`: an install or adopt
    /// must never pair one host's manifest with another host's blobs.
    pub(super) fn for_url(&self, url: &str) -> Option<&Manifest> {
        match &self.source {
            Some(src) if src == url => self.manifest.as_ref(),
            _ => None,
        }
    }

    pub(super) fn begin_fetch(&mut self, url: &str) {
        if self.source.as_deref() != Some(url) {
            self.manifest = None;
            self.source = None;
        }
        self.error = None;
        self.fetching = true;
    }
}

/// What a worker event asks of the app beyond this state.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Effects {
    /// Re-read `launcher-installed.json` and the client layout.
    pub refresh_install: bool,
}

#[derive(Debug)]
pub(super) struct PlayState {
    pub operation: Option<Operation>,
    pub game: Lifecycle,
    /// `SGW.exe` pids the process probe sees in the install folder.
    pub probed_pids: Vec<u32>,
    pub progress: Option<Progress>,
    pub notice: Option<Notice>,
    pub manifest: ManifestSlot,
}

impl Default for PlayState {
    fn default() -> Self {
        Self {
            operation: None,
            game: Lifecycle::Idle,
            probed_pids: Vec::new(),
            progress: None,
            notice: None,
            manifest: ManifestSlot::default(),
        }
    }
}

impl PlayState {
    /// The game as the surface shows it. A followed game wins; otherwise
    /// any `SGW.exe` the probe sees counts, with its diagnostics unknown.
    pub(super) fn activity(&self) -> GameActivity {
        match self.game {
            Lifecycle::Launching { .. } => GameActivity::Launching,
            Lifecycle::Running { telemetry, .. } => GameActivity::Running {
                telemetry: Some(telemetry),
            },
            Lifecycle::Idle if !self.probed_pids.is_empty() => {
                GameActivity::Running { telemetry: None }
            }
            Lifecycle::Idle => GameActivity::Idle,
        }
    }

    /// First-press feedback for Install / Update.
    pub(super) fn click_install(&mut self) {
        self.operation = Some(Operation::StartingInstall);
        self.progress = None;
        self.notice = Some(Notice::info("Starting…"));
    }

    pub(super) fn click_adopt(&mut self) {
        self.operation = Some(Operation::Adopting);
        self.notice = Some(Notice::info("Adopting the existing installation…"));
    }

    /// First-press feedback for Play. `telemetry` is the preference the
    /// launch is made with, so the footer can tell it from a later change.
    pub(super) fn click_play(&mut self, telemetry: bool) {
        self.game = Lifecycle::Launching { telemetry };
        self.notice = Some(Notice::info("Starting Stargate Worlds…"));
    }

    /// Apply a worker event. `manifest_url` is the URL the app uses now.
    pub(super) fn apply(&mut self, ev: &Event, manifest_url: &str) -> Effects {
        let mut fx = Effects::default();
        match ev {
            Event::ManifestFetched { url, manifest } => {
                if url == manifest_url {
                    self.manifest.manifest = Some(manifest.clone());
                    self.manifest.source = Some(url.clone());
                    self.manifest.error = None;
                    self.manifest.fetching = false;
                }
            }
            Event::ManifestError { url, message } => {
                if url == manifest_url {
                    self.manifest.error = Some(message.clone());
                    self.manifest.fetching = false;
                }
            }
            Event::Progress(p) => {
                if self.operation.is_some() {
                    self.progress = Some(p.clone());
                }
            }
            Event::InstallStarted => {
                self.operation = Some(Operation::Installing);
                self.notice = None;
            }
            Event::InstallComplete => {
                self.end_operation(Notice::info("Installation complete."));
                fx.refresh_install = true;
            }
            Event::InstallCancelled => {
                self.end_operation(Notice::info(
                    "Installation cancelled. What finished is kept; Install picks up from there.",
                ));
                fx.refresh_install = true;
            }
            Event::InstallError(e) => {
                self.end_operation(Notice::error(format!("Installation failed: {e}")));
                fx.refresh_install = true;
            }
            Event::AdoptComplete => {
                self.end_operation(Notice::info(
                    "Existing installation adopted. Its files were not verified.",
                ));
                fx.refresh_install = true;
            }
            Event::AdoptError(e) => {
                self.end_operation(Notice::error(format!("Could not adopt: {e}")));
                fx.refresh_install = true;
            }
            Event::Refused { action, reason } => {
                match action {
                    Busy::Install => self.operation = None,
                    Busy::Launch => {
                        if matches!(self.game, Lifecycle::Launching { .. }) {
                            self.game = Lifecycle::Idle;
                        }
                    }
                    Busy::Files => {}
                }
                self.notice = Some(Notice::error(format!("Not started: {reason}.")));
            }
            Event::Launched(_, pid) => {
                if let Lifecycle::Launching { telemetry } = self.game {
                    self.game = Lifecycle::Running {
                        pid: *pid,
                        telemetry,
                    };
                    self.notice = None;
                }
            }
            Event::LaunchError(e) => {
                self.game = Lifecycle::Idle;
                self.notice = Some(Notice::error(format!("Could not start the game: {e}")));
            }
            Event::GameExited { pid, exit_code } => {
                // The last probe may still list it; it has exited.
                self.probed_pids.retain(|p| p != pid);
                if matches!(self.game, Lifecycle::Running { pid: p, .. } if p == *pid) {
                    self.game = Lifecycle::Idle;
                    self.notice = Some(match exit_code {
                        Some(0) | None => Notice::info("The game closed."),
                        Some(code) => Notice::error(format!(
                            "The game closed with exit code {code}. If it crashed, \
                             Upload Debug Logs in Settings › Advanced helps us look."
                        )),
                    });
                }
            }
            Event::GameUntracked { pid } => {
                if matches!(self.game, Lifecycle::Running { pid: p, .. } if p == *pid) {
                    // The probe keeps showing it as running until it exits.
                    self.game = Lifecycle::Idle;
                }
            }
            _ => {}
        }
        fx
    }

    fn end_operation(&mut self, notice: Notice) {
        self.operation = None;
        self.progress = None;
        self.notice = Some(notice);
    }
}

/// How the install folder compares with the manifest and the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InstallStatus {
    NoFolder,
    /// `SGW.exe` is there but the launcher does not manage it yet.
    Adoptable,
    NotInstalled,
    NeedsUpdate {
        seed: bool,
        patches: usize,
    },
    UpToDate,
    /// Installed per the ledger, but no manifest to compare with.
    InstalledUnchecked,
}

/// Compare the ledger with the manifest. `adoptable` is an `SGW.exe`
/// with no ledger beside it ([`super::should_show_adopt_button`]).
pub(super) fn install_status(
    folder_set: bool,
    adoptable: bool,
    manifest: Option<&Manifest>,
    installed: &InstalledState,
) -> InstallStatus {
    if !folder_set {
        return InstallStatus::NoFolder;
    }
    if adoptable {
        return InstallStatus::Adoptable;
    }
    let Some(have) = installed.seed_sha256.as_deref() else {
        return InstallStatus::NotInstalled;
    };
    let Some(m) = manifest else {
        return InstallStatus::InstalledUnchecked;
    };
    let seed = have != m.seed.sha256;
    let patches = installed.missing_patches(&m.patches).len();
    if !seed && patches == 0 {
        InstallStatus::UpToDate
    } else {
        InstallStatus::NeedsUpdate { seed, patches }
    }
}

/// Facts about the install the surface needs besides [`PlayState`].
#[derive(Debug, Clone)]
pub(super) struct Inputs {
    pub status: InstallStatus,
    pub sgw_present: bool,
    pub writable: bool,
    pub launcher_blocked: bool,
}

/// The one big button, and what else the surface offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Primary {
    ChooseFolder,
    Install,
    Update,
    Adopt,
    Play,
    /// Install running; the button is Cancel.
    Installing,
    Busy(&'static str),
    Launching,
    Running,
    /// Nothing can be done here; the reason says why.
    Unavailable(String),
}

/// Decide the primary action. Order matters: a running job or game
/// always wins, then the launcher-version gate, then the install facts.
pub(super) fn primary_action(state: &PlayState, i: &Inputs) -> Primary {
    match state.operation {
        Some(Operation::StartingInstall | Operation::Installing) => return Primary::Installing,
        Some(Operation::Adopting) => return Primary::Busy("Adopting…"),
        None => {}
    }
    match state.activity() {
        GameActivity::Launching => return Primary::Launching,
        GameActivity::Running { .. } => return Primary::Running,
        GameActivity::Idle => {}
    }
    if i.launcher_blocked {
        return Primary::Unavailable(
            "This launcher is too old for the current game content. Update the launcher first."
                .into(),
        );
    }
    let need_writable = |p: Primary| {
        if i.writable {
            p
        } else {
            Primary::Unavailable(
                "The install folder is not writable. Choose another folder in Settings.".into(),
            )
        }
    };
    match &i.status {
        InstallStatus::NoFolder => Primary::ChooseFolder,
        InstallStatus::Adoptable => need_writable(Primary::Adopt),
        InstallStatus::NotInstalled => match (&state.manifest.manifest, &state.manifest.error) {
            (Some(_), _) => need_writable(Primary::Install),
            (None, Some(_)) => Primary::Unavailable(
                "Could not load the game content list. Check your connection and retry.".into(),
            ),
            (None, None) => Primary::Busy("Checking for game content…"),
        },
        InstallStatus::NeedsUpdate { .. } => need_writable(Primary::Update),
        InstallStatus::UpToDate | InstallStatus::InstalledUnchecked => {
            if i.sgw_present {
                Primary::Play
            } else {
                Primary::Unavailable(
                    "SGW.exe is missing, though the launcher recorded this folder as \
                     installed. Choose the folder that holds the game in Settings, or an \
                     empty folder to install again. Repair, in a later launcher update, \
                     will restore it in place."
                        .into(),
                )
            }
        }
    }
}

/// Whether "Play now" sits beside an Update button: the game is there,
/// so an update the player skips does not stop them playing.
pub(super) fn offers_play_anyway(primary: &Primary, i: &Inputs) -> bool {
    *primary == Primary::Update && i.sgw_present
}

/// Why a control that changes the install's files (install, adopt, the
/// install-folder change, client-state resets) is off, or `None`.
pub(super) fn file_action_block(state: &PlayState) -> Option<&'static str> {
    if state.operation.is_some() {
        return Some("Wait for the installation to finish.");
    }
    match state.activity() {
        GameActivity::Idle => None,
        GameActivity::Launching | GameActivity::Running { .. } => Some("Close the game first."),
    }
}

/// Why Play is off, or `None`.
pub(super) fn launch_block(state: &PlayState, i: &Inputs) -> Option<&'static str> {
    if state.operation.is_some() {
        return Some("Wait for the installation to finish.");
    }
    if state.activity() != GameActivity::Idle {
        return Some("The game is already running.");
    }
    if i.launcher_blocked {
        return Some("Update the launcher first.");
    }
    if !i.sgw_present {
        return Some("Install the game first.");
    }
    None
}

#[cfg(test)]
#[path = "play_state_tests.rs"]
mod tests;
