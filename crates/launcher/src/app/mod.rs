//! egui app: the launcher window's state and its lifecycle.
//!
//! The window follows the approved single-game design (#1153, layout A):
//! a gate panel on the left, and on the right one Install/Play surface,
//! a Patch Notes tab, a settings gear, and the diagnostics opt-in in the
//! footer of every view. This file holds [`LauncherApp`], its
//! construction, the event drain, and the click handlers that dispatch
//! worker commands. The rest:
//!
//! - [`play_state`]: the testable reducer behind the Install/Play surface
//!   and the guards every file-changing control asks.
//! - [`shell`]: the frame entry point, the side panel, header and tabs.
//! - [`play_tab`], [`patch_notes`], [`settings_panel`],
//!   [`advanced_panel`], [`telemetry_panel`]: the views.
//! - [`theme`] and [`gate_art`]: the palette and the gate motif.
//! - [`status_lines`]: event-to-text for the activity log.

mod advanced_panel;
mod client_changes_panel;
mod gate_art;
mod patch_notes;
mod play_state;
mod play_tab;
mod settings_panel;
mod shell;
mod status_lines;
mod telemetry_panel;
mod theme;
mod update_banner;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui;
use tokio::runtime::Runtime;

use crate::config::{config_path, LauncherConfig};
use crate::launch::LaunchOptions;
use crate::state::InstalledState;
use crate::worker::{Command, Event, LaunchSgwRequest, LaunchTelemetryConfig, Waker, Worker};
use play_state::{file_action_block, install_status, launch_block, Inputs, Notice, PlayState};
use settings_panel::PendingFolder;
use status_lines::{human_bytes, setup_status_lines, status_line_for};

/// Upper bound on the status-log history kept in memory. Display already
/// caps at the last 100 entries; this prevents the underlying Vec from
/// growing without bound during long sessions full of events.
const MAX_STATUS_LINES: usize = 1000;

/// How often the install ledger, the client layout and the game process
/// probe are re-read.
const REFRESH_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// The main column's tabs. Settings is a panel above either, not a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Play,
    Notes,
}

pub struct LauncherApp {
    config: LauncherConfig,
    /// The install folder as text, kept in step with
    /// `config.install_path` by the folder-change flow.
    install_path_text: String,
    /// Editable `Name = URL` lines for the login servers; parsed into
    /// `config.login_servers` on Save.
    login_servers_text: String,
    config_path: PathBuf,
    worker: Worker,
    status: Vec<String>,
    installed: InstalledState,
    launch_opts: LaunchOptions,
    /// Whether the install folder accepts writes, re-probed on refresh.
    writable: bool,
    last_refresh: std::time::Instant,
    /// The Install/Play surface's state: operation, game lifecycle,
    /// progress, the verified manifest.
    play: PlayState,
    tab: Tab,
    settings_open: bool,
    show_details: bool,
    /// The folder-change text box, while it is open.
    folder_edit: Option<String>,
    /// A checked folder waiting for "Use this folder".
    pending_folder: Option<PendingFolder>,
    /// The settings panel's last message: (is_error, text).
    settings_notice: Option<(bool, String)>,
    /// Set while the diagnostics choice could not be saved.
    telemetry_save_error: Option<String>,
    /// True while a confirm modal for "Reset all client state" is open.
    /// Higher-blast-radius wipe — gates the entire Firesky/ tree, not
    /// just the cache subdir — so we double-prompt before nuking.
    confirm_wipe_all_open: bool,
    /// Loaded once at app construction so each launch doesn't re-read
    /// install.json from disk.
    identity: Option<crate::identity::LauncherIdentity>,
    /// Launcher self-update state (banner, offer, `min_launcher` gate).
    update: update_banner::UpdateUi,
}

impl LauncherApp {
    /// `ctx` lets the worker wake the UI when a background job posts an
    /// event; without it a result waits for the next mouse move.
    pub fn new(runtime: Arc<Runtime>, ctx: egui::Context) -> Self {
        theme::apply(&ctx);
        let cp = config_path();
        let config = LauncherConfig::load(&cp).unwrap_or_default();
        let worker = Worker::new(runtime, repaint_waker(ctx));
        let install_path_text = config.install_path.to_string_lossy().into_owned();
        let login_servers_text = crate::client_setup::login_servers::to_text(&config.login_servers);
        let identity =
            crate::identity::LauncherIdentity::load_or_mint(&crate::identity::identity_path()).ok();
        let mut app = Self {
            config,
            install_path_text,
            login_servers_text,
            config_path: cp,
            worker,
            status: Vec::new(),
            installed: InstalledState::default(),
            launch_opts: LaunchOptions::default(),
            writable: true,
            last_refresh: std::time::Instant::now(),
            play: PlayState::default(),
            tab: Tab::Play,
            settings_open: false,
            show_details: false,
            folder_edit: None,
            pending_folder: None,
            settings_notice: None,
            telemetry_save_error: None,
            confirm_wipe_all_open: false,
            identity,
            update: update_banner::UpdateUi::new(crate::self_update::LauncherBuild::current()),
        };
        app.refresh_install_state();
        app.refresh_manifest();
        app.start_update_check();
        app
    }

    /// Append a status line, dropping the oldest entries when the buffer
    /// would exceed [`MAX_STATUS_LINES`]. The activity log only shows
    /// the last 100, so older drops are invisible.
    fn push_status(&mut self, line: String) {
        self.status.push(line);
        if self.status.len() > MAX_STATUS_LINES {
            let overflow = self.status.len() - MAX_STATUS_LINES;
            self.status.drain(0..overflow);
        }
    }

    /// Apply every queued worker event: the Play surface's reducer first,
    /// then the side effects it asks for, then the activity-log line.
    fn drain_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.worker.events_rx.try_recv() {
            let fx = self.play.apply(&ev, &self.config.manifest_url);
            if fx.refresh_install {
                self.refresh_install_state();
            }
            match &ev {
                Event::Update(u) => self.on_update_event(u, ctx),
                Event::OpenFolderError(e) => {
                    self.settings_notice = Some((true, format!("Could not open the folder: {e}")));
                }
                _ => {}
            }
            if let Some(line) = status_line_for(&ev) {
                self.push_status(line);
            }
            ctx.request_repaint();
        }
    }

    /// Re-read the ledger and client layout, re-probe writability, and
    /// look for a running game. Every [`REFRESH_EVERY`], and after any
    /// install or folder change.
    fn refresh_install_state(&mut self) {
        let path = self.config.install_path.clone();
        if path_is_empty(&path) {
            self.installed = InstalledState::default();
            self.launch_opts = LaunchOptions::default();
            self.writable = false;
        } else {
            self.installed = InstalledState::load(&path);
            self.launch_opts = LaunchOptions::detect(&crate::install_layout::binaries_dir(&path));
            self.writable = folder_writable(&path);
        }
        self.play.probed_pids = crate::game_process::running_game_pids(&path);
        self.last_refresh = std::time::Instant::now();
    }

    /// The install facts the Play surface and the guards read.
    fn inputs(&self) -> Inputs {
        let folder_set = !path_is_empty(&self.config.install_path);
        Inputs {
            status: install_status(
                folder_set,
                folder_set && should_show_adopt_button(&self.config.install_path),
                self.play.manifest.manifest.as_ref(),
                &self.installed,
            ),
            sgw_present: self.launch_opts.sgw_present,
            writable: self.writable,
            launcher_blocked: self.launcher_blocked(),
        }
    }

    /// Fetch the manifest for the URL in use. Only a manifest whose
    /// signature verified ever reaches the UI.
    fn refresh_manifest(&mut self) {
        let url = self.config.manifest_url.clone();
        self.play.manifest.begin_fetch(&url);
        self.worker.fetch_manifest_now(url);
    }

    /// Parse the login-server text, save the config, and apply the new
    /// list to an existing install right away.
    fn save_config(&mut self) {
        match crate::client_setup::login_servers::parse(&self.login_servers_text) {
            Ok(servers) => self.config.login_servers = servers,
            Err(e) => {
                self.push_status(format!("Not saved: login servers: {e}"));
                return;
            }
        }
        match self.config.save(&self.config_path) {
            Ok(_) => {
                self.push_status("Saved config.".into());
                self.refresh_install_state();
                if self.launch_opts.sgw_present && file_action_block(&self.play).is_none() {
                    self.prepare_client_for_launch();
                }
            }
            Err(e) => self.push_status(format!("Save failed: {e}")),
        }
    }

    /// Install or update from the verified manifest.
    fn start_install(&mut self) {
        let Some(manifest) = self.play.manifest.manifest.clone() else {
            return;
        };
        if let Some(why) = file_action_block(&self.play) {
            self.push_status(format!("Not started: {why}"));
            return;
        }
        if let Err(e) = self.config.save(&self.config_path) {
            self.push_status(format!("Save failed: {e}"));
        }
        self.play.click_install();
        self.push_status("Starting install / update…".into());
        self.worker.dispatch(Command::Install {
            config: self.config.clone(),
            manifest,
        });
    }

    fn cancel_install(&mut self) {
        self.worker.dispatch(Command::Cancel);
        self.push_status("Cancel requested.".into());
        self.play.notice = Some(Notice {
            error: false,
            text: "Cancelling… the current step finishes first.".into(),
        });
    }

    /// Mark the folder's existing client as launcher-managed.
    fn start_adopt(&mut self) {
        let Some(manifest) = self.play.manifest.manifest.clone() else {
            return;
        };
        if file_action_block(&self.play).is_some() {
            return;
        }
        self.play.click_adopt();
        self.push_status("Adopt requested…".into());
        self.worker.dispatch(Command::AdoptExisting {
            install_dir: self.config.install_path.clone(),
            manifest,
        });
    }

    /// Play: client setup, then the launch with the client patches and,
    /// when opted in, telemetry. The surface shows "Starting…" at once.
    fn start_play(&mut self) {
        if let Some(why) = launch_block(&self.play, &self.inputs()) {
            self.push_status(format!("Not launching: {why}"));
            return;
        }
        if !self.prepare_client_for_launch() {
            self.play.launch_aborted(
                "Could not prepare the game files; see Settings › Advanced › Activity log.".into(),
            );
            return;
        }
        // Telemetry follows the game only when the player opted in and
        // the identity loaded; the client patches are independent.
        let telemetry = match &self.identity {
            Some(id) if self.config.telemetry.opted_in => {
                Some(build_telemetry_config(&self.config, id))
            }
            _ => None,
        };
        self.play.click_play(telemetry.is_some());
        self.worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
            install_dir: crate::install_layout::binaries_dir(&self.config.install_path),
            client_patches: self.config.client_patches.clone(),
            telemetry,
        }));
    }

    /// Write `LoginInternal.lua` and switch ASLR off before a launch.
    /// Returns false when setup failed; callers then don't launch, since a
    /// client with ASLR still on breaks the patches DLL and the RE
    /// addresses, and one without the server list can't log in.
    fn prepare_client_for_launch(&mut self) -> bool {
        let result =
            crate::client_setup::prepare(&self.config.install_path, &self.config.login_servers);
        let (ok, lines) = setup_status_lines(&result);
        for line in lines {
            self.push_status(line);
        }
        ok
    }
}

/// The worker's wake hook: one repaint per event. `request_repaint` is
/// thread-safe and wakes eframe's event loop, so the next frame drains
/// the channel (see `crate::worker::EventSender`).
fn repaint_waker(ctx: egui::Context) -> Waker {
    Arc::new(move || ctx.request_repaint())
}

/// True iff `p` is an empty path (no components). Replaces the
/// `String::is_empty()` checks from before the PathBuf migration.
fn path_is_empty(p: &Path) -> bool {
    p.as_os_str().is_empty()
}

fn build_telemetry_config(
    config: &LauncherConfig,
    identity: &crate::identity::LauncherIdentity,
) -> LaunchTelemetryConfig {
    LaunchTelemetryConfig {
        auth_base_url: config.telemetry.auth_url.clone(),
        install_id: identity.install_id.to_string(),
        machine_id: identity.machine_id.clone(),
        // Built-in: a packaged launcher's build env carries the
        // branch + git_sha at compile time. For now the OptionEnv!s
        // default to "dev"/"unknown" — the release workflow can set
        // CIMMERIA_BUILD_BRANCH / CIMMERIA_BUILD_GIT_SHA at compile
        // time.
        branch: option_env!("CIMMERIA_BUILD_BRANCH").unwrap_or("dev").into(),
        git_sha: option_env!("CIMMERIA_BUILD_GIT_SHA")
            .unwrap_or("unknown")
            .into(),
        launcher_version: identity.created_by_launcher_version.clone(),
        state_dir: crate::config::exe_dir(),
        tags: vec![],
        login_server_urls: config.login_servers.iter().map(|s| s.url.clone()).collect(),
    }
}

/// Whether an install could write to `dir`, without creating it: a
/// missing folder is judged by its nearest existing parent. The probe
/// runs every refresh, so it must not make the folder appear on its own.
fn folder_writable(dir: &Path) -> bool {
    let mut probe = dir;
    while !probe.exists() {
        match probe.parent() {
            Some(p) if !p.as_os_str().is_empty() => probe = p,
            _ => return false,
        }
    }
    probe.is_dir() && crate::launch::install_dir_writable(probe)
}

/// Whether to surface the "Adopt existing install" affordance.
///
/// True iff the install at `install_path` has `SGW.exe` (at the top, or in
/// `Working\Binaries`; see [`crate::install_layout`]) AND does NOT contain a
/// `launcher-installed.json` marker file. The first condition rules
/// out empty directories (those should go through the normal Install
/// path); the second condition rules out installs the launcher
/// already manages (those have nothing to adopt). Extracted so the
/// boolean decision is unit-testable without spinning up an egui frame.
fn should_show_adopt_button(install_path: &Path) -> bool {
    crate::install_layout::sgw_exe(install_path).is_file()
        && !crate::state::InstalledState::path(install_path).exists()
}

#[cfg(test)]
mod tests {
    use super::folder_writable;

    // Bug shape: the 2-second refresh probed writability by creating the
    // install folder, so any configured path appeared on disk unasked.
    #[test]
    fn the_writability_probe_does_not_create_the_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("a").join("Stargate Worlds");
        assert!(folder_writable(&target));
        assert!(!tmp.path().join("a").exists(), "nothing may be created");
        assert!(folder_writable(tmp.path()));
    }
}
