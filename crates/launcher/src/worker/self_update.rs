//! Worker tasks for the launcher self-update: the background check and the
//! one-click download + swap + relaunch. The logic is in
//! [`crate::self_update`]; this file turns it into [`UpdateEvent`]s.

use tokio::sync::mpsc;

use super::{Event, Worker};
use crate::install::Progress;
use crate::self_update::{self, CheckOutcome, LauncherBuild, LauncherRelease, UpdateEndpoints};

/// Self-update events, wrapped in [`Event::Update`].
#[derive(Debug, Clone)]
pub enum UpdateEvent {
    Checked(CheckOutcome),
    /// Offline, rate limited, or GitHub answered with an error. Quiet: a
    /// status line, never a dialog.
    CheckFailed {
        message: String,
    },
    Progress {
        downloaded: u64,
        total: u64,
    },
    /// The update did not happen. `page_url` is the manual fallback.
    Failed {
        message: String,
        page_url: String,
    },
    /// The new launcher is running; this one should close now. In
    /// production the handoff exits the process before this is sent, so
    /// the UI's close on it is only a backstop.
    Restarting {
        tag: String,
    },
}

/// The updater's own client, endpoints and build identity.
pub(super) struct Updater {
    pub(super) http: reqwest::Client,
    pub(super) endpoints: UpdateEndpoints,
    pub(super) build: LauncherBuild,
}

impl Updater {
    pub(super) fn production() -> Self {
        let build = LauncherBuild::current();
        let endpoints = UpdateEndpoints::github();
        let http = endpoints
            .client(&self_update::user_agent(&build))
            .expect("build the updater HTTP client");
        Self {
            http,
            endpoints,
            build,
        }
    }
}

impl Worker {
    pub(super) fn spawn_update_check(&self) {
        let events_tx = self.events_tx.clone();
        let http = self.updater.http.clone();
        let endpoints = self.updater.endpoints.clone();
        let build = self.updater.build.clone();
        self.runtime.spawn(async move {
            let ev = match self_update::check(&http, &endpoints, &build).await {
                Ok(out) => UpdateEvent::Checked(out),
                Err(e) => UpdateEvent::CheckFailed {
                    message: e.to_string(),
                },
            };
            let _ = events_tx.send(Event::Update(ev));
        });
    }

    pub(super) fn spawn_update_apply(&self, release: LauncherRelease) {
        let events_tx = self.events_tx.clone();
        let fwd_tx = self.events_tx.clone();
        let http = self.updater.http.clone();
        let endpoints = self.updater.endpoints.clone();
        let build = self.updater.build.clone();
        let (prog_tx, mut prog_rx) = mpsc::unbounded_channel::<Progress>();
        self.runtime.spawn(async move {
            while let Some(p) = prog_rx.recv().await {
                if let Progress::Downloading {
                    downloaded, total, ..
                } = p
                {
                    let _ = fwd_tx.send(Event::Update(UpdateEvent::Progress { downloaded, total }));
                }
            }
        });
        self.runtime.spawn(async move {
            let ev = match std::env::current_exe() {
                Err(e) => failed(&release, &self_update::ApplyError::NoExePath(e)),
                Ok(exe) => {
                    // On success the handoff releases launcher.lock and
                    // exits the process from this task; it does not wait
                    // for an egui frame (see self_update::handoff).
                    let mut hooks = self_update::handoff::ProcessHooks;
                    match self_update::apply(
                        &http, &endpoints, &build, &release, &exe, &prog_tx, &mut hooks,
                    )
                    .await
                    {
                        Ok(()) => UpdateEvent::Restarting {
                            tag: release.tag.clone(),
                        },
                        Err(e) => failed(&release, &e),
                    }
                }
            };
            let _ = events_tx.send(Event::Update(ev));
        });
    }
}

fn failed(release: &LauncherRelease, e: &self_update::ApplyError) -> UpdateEvent {
    UpdateEvent::Failed {
        message: e.to_string(),
        page_url: release.page_url.clone(),
    }
}
