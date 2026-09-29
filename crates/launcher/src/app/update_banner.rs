//! The launcher self-update banner at the top of the window, its state,
//! and the manifest's `min_launcher` gate on Install / Update and Launch.
//!
//! The check runs once in the background at startup. When a newer
//! launcher release exists, the banner offers "Update now"; one click
//! downloads, verifies, swaps and relaunches. Failures are shown in plain
//! words with the release page as the manual fallback.

use eframe::egui;

use super::{human_bytes, LauncherApp};
use crate::self_update::swap::RELAUNCH_ENV;
use crate::self_update::{
    version, LauncherBuild, LauncherRelease, MinLauncherGate, UpdateDecision, UpdateEndpoints,
};
use crate::worker::{Command, UpdateEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UpdatePhase {
    Idle,
    Checking,
    Downloading { downloaded: u64, total: u64 },
    Restarting { tag: String },
    Failed { message: String, page_url: String },
}

/// Self-update state held by [`LauncherApp`].
#[derive(Debug)]
pub(super) struct UpdateUi {
    pub(super) build: LauncherBuild,
    pub(super) phase: UpdatePhase,
    /// The newer release on offer, if the last check found one.
    pub(super) offer: Option<LauncherRelease>,
    /// Every candidate from the last successful check, for the
    /// `min_launcher` same-day lookup.
    pub(super) known: Vec<LauncherRelease>,
}

impl UpdateUi {
    pub(super) fn new(build: LauncherBuild) -> Self {
        let phase = if build.is_release() {
            UpdatePhase::Checking
        } else {
            UpdatePhase::Idle
        };
        Self {
            build,
            phase,
            offer: None,
            known: Vec::new(),
        }
    }

    /// Apply a worker event to the banner state.
    pub(super) fn apply(&mut self, ev: &UpdateEvent) {
        match ev {
            UpdateEvent::Checked(out) => {
                self.known = out.releases.clone();
                self.offer = match &out.decision {
                    UpdateDecision::Available(r) => Some(r.clone()),
                    _ => None,
                };
                if self.phase == UpdatePhase::Checking {
                    self.phase = UpdatePhase::Idle;
                }
            }
            UpdateEvent::CheckFailed { .. } => {
                if self.phase == UpdatePhase::Checking {
                    self.phase = UpdatePhase::Idle;
                }
            }
            UpdateEvent::Progress { downloaded, total } => {
                if matches!(self.phase, UpdatePhase::Downloading { .. }) {
                    self.phase = UpdatePhase::Downloading {
                        downloaded: *downloaded,
                        total: *total,
                    };
                }
            }
            UpdateEvent::Failed { message, page_url } => {
                self.phase = UpdatePhase::Failed {
                    message: message.clone(),
                    page_url: page_url.clone(),
                };
            }
            UpdateEvent::Restarting { tag } => {
                self.phase = UpdatePhase::Restarting { tag: tag.clone() };
            }
        }
    }

    /// The `min_launcher` gate for the manifest's value.
    pub(super) fn gate(&self, min_launcher: Option<&str>) -> MinLauncherGate {
        version::check_min_launcher(&self.build, min_launcher, &self.known)
    }

    /// True while a download or restart is under way.
    pub(super) fn busy(&self) -> bool {
        matches!(
            self.phase,
            UpdatePhase::Checking
                | UpdatePhase::Downloading { .. }
                | UpdatePhase::Restarting { .. }
        )
    }
}

/// The status-log line for an update event; `None` for progress ticks.
pub(super) fn update_status_line(ev: &UpdateEvent) -> Option<String> {
    Some(match ev {
        UpdateEvent::Checked(out) => match &out.decision {
            UpdateDecision::DevBuild => "Development build: launcher updates are disabled.".into(),
            UpdateDecision::NoRelease => "No launcher release to update from was found.".into(),
            UpdateDecision::UpToDate { newest } => {
                format!("Launcher is up to date (newest release: {newest}).")
            }
            UpdateDecision::Available(r) => format!("Launcher update available: {}.", r.tag),
        },
        UpdateEvent::CheckFailed { message, .. } => {
            format!("Could not check for launcher updates: {message}.")
        }
        UpdateEvent::Progress { .. } => return None,
        UpdateEvent::Failed { message, page_url } => {
            format!("Launcher update failed: {message}. Download it by hand from {page_url}")
        }
        UpdateEvent::Restarting { tag } => format!("Launcher updated to {tag}; restarting…"),
    })
}

/// The status line a relaunched launcher starts with, from
/// [`RELAUNCH_ENV`].
pub(super) fn relaunch_greeting(from: Option<String>, build: &LauncherBuild) -> Option<String> {
    let from = from.filter(|f| !f.is_empty())?;
    Some(format!(
        "Launcher updated from {from} to {}.",
        build.display()
    ))
}

impl LauncherApp {
    /// Startup: say if this start follows an update, and kick off the
    /// background check. Called once from [`LauncherApp::new`].
    pub(super) fn start_update_check(&mut self) {
        if let Some(line) = relaunch_greeting(std::env::var(RELAUNCH_ENV).ok(), &self.update.build)
        {
            self.push_status(line);
        }
        if self.update.build.is_release() {
            self.worker.dispatch(Command::CheckForUpdate);
        }
    }

    pub(super) fn on_update_event(&mut self, ev: &UpdateEvent, ctx: &egui::Context) {
        self.update.apply(ev);
        if let UpdateEvent::Restarting { .. } = ev {
            // Backstop only: the handoff has already released the lock
            // and exited the process from the worker (a close here waits
            // for a frame, which is what left 676f314 holding the lock).
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// True when the manifest's `min_launcher` says this launcher is too
    /// old: Install / Update and Launch are then off.
    pub(super) fn launcher_blocked(&self) -> bool {
        self.min_gate().blocks()
    }

    fn min_gate(&self) -> MinLauncherGate {
        self.update.gate(
            self.manifest
                .as_ref()
                .and_then(|m| m.min_launcher.as_deref()),
        )
    }

    fn click_update(&mut self, release: LauncherRelease) {
        // First-press feedback: the banner switches to a progress bar and
        // the status log says what is happening before any byte arrives.
        self.update.phase = UpdatePhase::Downloading {
            downloaded: 0,
            total: release.exe.size,
        };
        self.push_status(format!(
            "Downloading launcher update {} ({})…",
            release.tag,
            human_bytes(release.exe.size)
        ));
        self.worker.dispatch(Command::ApplyUpdate(release));
    }

    fn click_check(&mut self) {
        self.update.phase = UpdatePhase::Checking;
        self.push_status("Checking for launcher updates…".into());
        self.worker.dispatch(Command::CheckForUpdate);
    }

    pub(super) fn show_update_banner(&mut self, ui: &mut egui::Ui) {
        let gate = self.min_gate();
        if let MinLauncherGate::TooOld { required } = &gate {
            ui.colored_label(
                egui::Color32::RED,
                format!(
                    "This launcher ({}) is too old for the server's content; it needs {required} \
                     or newer. Install and Launch are off until you update.",
                    self.update.build.display()
                ),
            );
        }

        match self.update.phase.clone() {
            UpdatePhase::Downloading { downloaded, total } => {
                let pct = if total > 0 {
                    (downloaded as f32 / total as f32).min(1.0)
                } else {
                    0.0
                };
                ui.label(format!(
                    "Downloading launcher update: {} / {}",
                    human_bytes(downloaded),
                    human_bytes(total)
                ));
                ui.add(egui::ProgressBar::new(pct).show_percentage());
            }
            UpdatePhase::Restarting { tag } => {
                ui.colored_label(
                    egui::Color32::LIGHT_GREEN,
                    format!("Update verified. Restarting into {tag}…"),
                );
            }
            UpdatePhase::Failed { message, page_url } => {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Launcher update failed: {message}."),
                );
                ui.horizontal(|ui| {
                    ui.hyperlink_to("Download the new launcher from the release page", page_url);
                    if let Some(offer) = self.update.offer.clone() {
                        if ui
                            .add_enabled(!self.installing, egui::Button::new("Try again"))
                            .clicked()
                        {
                            self.click_update(offer);
                        }
                    }
                });
            }
            UpdatePhase::Idle | UpdatePhase::Checking => {
                if let Some(offer) = self.update.offer.clone() {
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            egui::Color32::LIGHT_YELLOW,
                            format!("Launcher update available ({})", offer.tag),
                        );
                        if ui
                            .add_enabled(!self.installing, egui::Button::new("Update now"))
                            .on_disabled_hover_text("Wait for the install to finish.")
                            .clicked()
                        {
                            self.click_update(offer.clone());
                        }
                        ui.hyperlink_to("release notes", &offer.page_url);
                    });
                } else if gate.blocks() {
                    ui.hyperlink_to(
                        "Download the new launcher from the releases page",
                        UpdateEndpoints::releases_page(),
                    );
                }
            }
        }

        ui.horizontal(|ui| {
            let version = if self.update.build.is_release() {
                format!("Launcher {}", self.update.build.display())
            } else {
                "Launcher: development build — updates disabled".into()
            };
            ui.label(egui::RichText::new(version).small());
            if self.update.build.is_release() {
                let label = if self.update.phase == UpdatePhase::Checking {
                    "Checking…"
                } else {
                    "Check for updates"
                };
                if ui
                    .add_enabled(!self.update.busy(), egui::Button::new(label).small())
                    .clicked()
                {
                    self.click_check();
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_update::releases::ReleaseAsset;
    use crate::self_update::CheckOutcome;

    fn rel(tag: &str) -> LauncherRelease {
        let a = |n: &str| ReleaseAsset {
            name: n.into(),
            url: format!("https://github.com/x/{n}"),
            size: 10,
        };
        LauncherRelease {
            tag: tag.into(),
            published_at: 2600,
            page_url: UpdateEndpoints::release_page(tag),
            exe: a("x.exe"),
            sha256: a("x.exe.sha256"),
        }
    }

    fn release_build() -> LauncherBuild {
        LauncherBuild::from_parts(Some("launcher-20261002-aaaaaaa"), Some("1000"))
    }

    #[test]
    fn a_check_with_an_update_puts_it_on_offer() {
        let mut ui = UpdateUi::new(release_build());
        assert_eq!(ui.phase, UpdatePhase::Checking, "startup check is visible");
        let r = rel("launcher-20261002-bbbbbbb");
        ui.apply(&UpdateEvent::Checked(CheckOutcome {
            decision: UpdateDecision::Available(r.clone()),
            releases: vec![r.clone()],
        }));
        assert_eq!(ui.offer, Some(r));
        assert_eq!(ui.phase, UpdatePhase::Idle);
    }

    #[test]
    fn a_dev_build_shows_no_check_and_no_offer() {
        let ui = UpdateUi::new(LauncherBuild::dev());
        assert_eq!(ui.phase, UpdatePhase::Idle);
        assert!(ui.offer.is_none());
        assert!(!ui.gate(Some("launcher-20991231-fffffff")).blocks());
    }

    // Offline and rate limiting are quiet: the phase settles and the
    // status log gets one plain line.
    #[test]
    fn a_failed_check_settles_quietly() {
        let mut ui = UpdateUi::new(release_build());
        let ev = UpdateEvent::CheckFailed {
            message: "GitHub rate limit reached; the launcher will check again next start".into(),
        };
        ui.apply(&ev);
        assert_eq!(ui.phase, UpdatePhase::Idle);
        assert!(update_status_line(&ev)
            .unwrap()
            .starts_with("Could not check"));
    }

    #[test]
    fn a_failed_update_keeps_the_release_page_link() {
        let mut ui = UpdateUi::new(release_build());
        ui.phase = UpdatePhase::Downloading {
            downloaded: 0,
            total: 10,
        };
        let page = UpdateEndpoints::release_page("launcher-20261002-bbbbbbb");
        let ev = UpdateEvent::Failed {
            message: "the download does not match the release's SHA-256".into(),
            page_url: page.clone(),
        };
        ui.apply(&ev);
        assert!(matches!(&ui.phase, UpdatePhase::Failed { page_url, .. } if *page_url == page));
        assert!(update_status_line(&ev).unwrap().contains(&page));
    }

    #[test]
    fn progress_only_moves_an_active_download() {
        let mut ui = UpdateUi::new(release_build());
        ui.phase = UpdatePhase::Idle;
        ui.apply(&UpdateEvent::Progress {
            downloaded: 5,
            total: 10,
        });
        assert_eq!(ui.phase, UpdatePhase::Idle);
        ui.phase = UpdatePhase::Downloading {
            downloaded: 0,
            total: 10,
        };
        ui.apply(&UpdateEvent::Progress {
            downloaded: 5,
            total: 10,
        });
        assert_eq!(
            ui.phase,
            UpdatePhase::Downloading {
                downloaded: 5,
                total: 10
            }
        );
        assert!(update_status_line(&UpdateEvent::Progress {
            downloaded: 5,
            total: 10
        })
        .is_none());
    }

    #[test]
    fn min_launcher_gate_uses_the_checked_release_list() {
        let mut ui = UpdateUi::new(release_build());
        let b = rel("launcher-20261002-bbbbbbb");
        assert!(
            !ui.gate(Some(&b.tag)).blocks(),
            "same day, unknown: let through"
        );
        ui.apply(&UpdateEvent::Checked(CheckOutcome {
            decision: UpdateDecision::Available(b.clone()),
            releases: vec![b.clone()],
        }));
        assert!(
            ui.gate(Some(&b.tag)).blocks(),
            "b was published after this build"
        );
        assert!(!ui.gate(None).blocks(), "no min_launcher, no gate");
    }

    #[test]
    fn a_relaunched_launcher_says_what_it_updated_from() {
        let b = LauncherBuild::from_parts(Some("launcher-20261002-bbbbbbb"), Some("2000"));
        assert_eq!(
            relaunch_greeting(Some("launcher-20261002-aaaaaaa".into()), &b).unwrap(),
            "Launcher updated from launcher-20261002-aaaaaaa to launcher-20261002-bbbbbbb."
        );
        assert!(relaunch_greeting(None, &b).is_none());
        assert!(relaunch_greeting(Some(String::new()), &b).is_none());
    }
}
