//! egui app — top-level UI state machine and panels.
//!
//! [`mod.rs`](self) holds the [`LauncherApp`] state struct, its
//! construction + event-drain lifecycle, and the pure helper functions
//! ([`status_line_for`], [`human_bytes`], [`should_show_adopt_button`])
//! that are unit-tested without an egui frame. The panel rendering — the
//! `eframe::App` impl and every `show_*` method — lives in
//! [`view`](self::view).

mod view;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui;
use tokio::runtime::Runtime;

use crate::config::{config_path, LauncherConfig};
use crate::install::Progress;
use crate::launch::LaunchOptions;
use crate::manifest::Manifest;
use crate::state::InstalledState;
use crate::worker::{Event, LaunchTelemetryConfig, Worker};

/// Upper bound on the status-log history kept in memory. Display already
/// caps at the last 100 entries; this prevents the underlying Vec from
/// growing without bound during long sessions full of events.
const MAX_STATUS_LINES: usize = 1000;

pub struct LauncherApp {
    config: LauncherConfig,
    /// Editable text buffer for the install-dir TextEdit widget. egui's
    /// `TextEdit::singleline` takes `&mut String`, but the persisted
    /// config field is `PathBuf` — this is the sync target.
    install_path_text: String,
    /// Editable `Name = URL` lines for the login servers; parsed into
    /// `config.login_servers` on Save.
    login_servers_text: String,
    config_path: PathBuf,
    worker: Worker,
    last_progress: Option<Progress>,
    status: Vec<String>,
    manifest: Option<Manifest>,
    manifest_error: Option<String>,
    installed: InstalledState,
    launch_opts: LaunchOptions,
    last_refresh: std::time::Instant,
    installing: bool,
    /// True while a confirm modal for "Reset all client state" is open.
    /// Higher-blast-radius wipe — gates the entire Firesky/ tree, not
    /// just the cache subdir — so we double-prompt before nuking.
    confirm_wipe_all_open: bool,
    /// Loaded once at app construction so each Launch+Telemetry click
    /// doesn't re-read install.json from disk.
    identity: Option<crate::identity::LauncherIdentity>,
}

impl LauncherApp {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        let cp = config_path();
        let config = LauncherConfig::load(&cp).unwrap_or_default();
        let worker = Worker::new(runtime);
        let installed = if path_is_empty(&config.install_path) {
            InstalledState::default()
        } else {
            InstalledState::load(&config.install_path)
        };
        let launch_opts = if path_is_empty(&config.install_path) {
            LaunchOptions::default()
        } else {
            LaunchOptions::detect(&crate::install_layout::binaries_dir(&config.install_path))
        };
        worker.fetch_manifest_now(config.manifest_url.clone());
        let install_path_text = config.install_path.to_string_lossy().into_owned();
        let login_servers_text = crate::client_setup::login_servers::to_text(&config.login_servers);
        let identity =
            crate::identity::LauncherIdentity::load_or_mint(&crate::identity::identity_path()).ok();
        Self {
            config,
            install_path_text,
            login_servers_text,
            config_path: cp,
            worker,
            last_progress: None,
            status: Vec::new(),
            manifest: None,
            manifest_error: None,
            installed,
            launch_opts,
            last_refresh: std::time::Instant::now(),
            installing: false,
            confirm_wipe_all_open: false,
            identity,
        }
    }

    /// Pull the latest text-buffer value into the persisted PathBuf so
    /// the save path / install_dir comparisons all see the user's edit.
    fn sync_install_path_from_text(&mut self) {
        self.config.install_path = PathBuf::from(&self.install_path_text);
    }

    /// Append a status line, dropping the oldest entries when the buffer
    /// would exceed [`MAX_STATUS_LINES`]. The display only ever reads the
    /// most-recent 100 entries (see [`Self::show_status_log`]) so older
    /// drops are invisible to the user.
    fn push_status(&mut self, line: String) {
        self.status.push(line);
        if self.status.len() > MAX_STATUS_LINES {
            let overflow = self.status.len() - MAX_STATUS_LINES;
            self.status.drain(0..overflow);
        }
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.worker.events_rx.try_recv() {
            // Events that drive *non-status* UI state (manifest panel,
            // progress bars, the installing/managed-install flags) get
            // their side effects applied here. The status-log line — if
            // any — comes from `status_line_for`, which is the single
            // source of truth for ev-to-text translation and is unit
            // tested directly.
            match &ev {
                Event::ManifestFetched(m) => {
                    self.manifest = Some(m.clone());
                    self.manifest_error = None;
                }
                Event::ManifestError(e) => {
                    self.manifest_error = Some(e.clone());
                }
                Event::Progress(p) => {
                    self.last_progress = Some(p.clone());
                }
                Event::InstallComplete => {
                    self.installing = false;
                    self.refresh_install_state();
                }
                Event::InstallError(_) => {
                    self.installing = false;
                }
                Event::AdoptComplete => {
                    self.refresh_install_state();
                }
                Event::AdoptError(_)
                | Event::Wiped { .. }
                | Event::WipeError(_)
                | Event::Launched(..)
                | Event::LaunchError(_)
                | Event::ClientPatchesNote(_)
                | Event::UploadStarted
                | Event::UploadSkipped(_)
                | Event::UploadComplete { .. }
                | Event::UploadError(_)
                | Event::TelemetrySessionComplete(_)
                | Event::TelemetrySessionError(_) => {
                    // Status-only events — handled below.
                }
            }
            if let Some(line) = status_line_for(&ev) {
                self.push_status(line);
            }
            ctx.request_repaint();
        }
    }

    fn refresh_install_state(&mut self) {
        if !path_is_empty(&self.config.install_path) {
            let path = self.config.install_path.as_path();
            self.installed = InstalledState::load(path);
            self.launch_opts = LaunchOptions::detect(&crate::install_layout::binaries_dir(path));
        } else {
            self.installed = InstalledState::default();
            self.launch_opts = LaunchOptions::default();
        }
    }

    /// Parse the login-server text, save the config, and apply the new
    /// list to an existing install right away.
    fn save_config(&mut self) {
        self.sync_install_path_from_text();
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
                if self.launch_opts.sgw_present {
                    self.prepare_client_for_launch();
                }
            }
            Err(e) => self.push_status(format!("Save failed: {e}")),
        }
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

/// Status lines for a client-setup result, and whether launching may go
/// ahead. Extracted so the launch gate is testable without an egui frame.
fn setup_status_lines(
    result: &std::io::Result<crate::client_setup::SetupReport>,
) -> (bool, Vec<String>) {
    match result {
        Ok(report) => {
            let mut lines = Vec::new();
            if report.login_servers_written {
                lines.push("Wrote the login server list (LoginInternal.lua).".into());
            }
            if report.aslr == crate::client_setup::AslrOutcome::Disabled {
                lines.push("Switched ASLR off in SGW.exe.".into());
            }
            (true, lines)
        }
        Err(e) => (
            false,
            vec![format!("Not launching: client setup failed: {e}")],
        ),
    }
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
    }
}

/// Whether to surface the "Adopt existing install" affordance.
///
/// True iff the install at `install_path` has `SGW.exe` (at the top, or in
/// `Working\Binaries`; see [`crate::install_layout`]) AND does NOT contain a
/// `launcher-installed.json` marker file. The first condition rules
/// out empty directories (those should go through the normal Install
/// path); the second condition rules out installs the launcher
/// already manages (those have nothing to adopt). Extracted from
/// `show_install_panel` so the boolean decision is unit-testable
/// without spinning up an egui frame.
fn should_show_adopt_button(install_path: &Path) -> bool {
    crate::install_layout::sgw_exe(install_path).is_file()
        && !crate::state::InstalledState::path(install_path).exists()
}

/// Render a worker [`Event`] into the human-readable status-log line
/// the UI appends to its scrollback. Pure formatting — extracted from
/// `drain_events` so each Event arm has at least minimal coverage
/// without needing an egui context. Returns `None` for events that
/// don't translate to a status line on their own (manifest updates,
/// progress ticks).
fn status_line_for(event: &Event) -> Option<String> {
    Some(match event {
        Event::AdoptComplete => {
            "Adopted existing install — patches will apply on top (seed bytes not verified).".into()
        }
        Event::AdoptError(e) => format!("Adopt failed: {e}"),
        Event::Wiped { kind, report } => format!(
            "Wiped {kind}: {} item(s), {} freed",
            report.entries_removed,
            human_bytes(report.bytes_freed)
        ),
        Event::WipeError(e) => format!("Wipe failed: {e}"),
        Event::InstallComplete => "Install complete.".into(),
        Event::InstallError(e) => format!("Install failed: {e}"),
        Event::Launched(name, pid) => format!("Launched {name} (pid {pid})"),
        Event::LaunchError(e) => format!("Launch failed: {e}"),
        Event::ClientPatchesNote(n) => format!("Client patches: {n}"),
        Event::UploadStarted => "Uploading logs…".into(),
        Event::UploadSkipped(why) => format!("Log upload skipped: {why}"),
        Event::UploadComplete { blob, bytes } => format!("Uploaded {bytes} bytes to {blob}"),
        Event::UploadError(e) => format!("Log upload failed: {e}"),
        Event::TelemetrySessionComplete(o) => {
            let sha_short: String = o.bundle_sha256.chars().take(12).collect();
            format!(
                "Telemetry session complete — {} events, {} dropped, bundle {} (sha {})",
                o.event_count,
                o.dropped_lines,
                human_bytes(o.bundle_bytes),
                if sha_short.is_empty() {
                    "n/a"
                } else {
                    sha_short.as_str()
                }
            )
        }
        Event::TelemetrySessionError(e) => format!("Telemetry session error: {e}"),
        // Progress + manifest events drive other UI state, not the
        // status log. Returning None makes that explicit.
        Event::ManifestFetched(_) | Event::ManifestError(_) | Event::Progress(_) => return None,
    })
}

fn human_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut f = n as f64;
    let mut i = 0;
    while f >= 1024.0 && i + 1 < UNITS.len() {
        f /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{f:.2} {}", UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::{
        human_bytes, setup_status_lines, should_show_adopt_button, status_line_for,
        MAX_STATUS_LINES,
    };
    use crate::client_setup::{AslrOutcome, SetupReport};

    // Bug shape: a failed client setup (SGW.exe locked, ASLR still on) used
    // to be reported and then launched anyway.
    #[test]
    fn a_failed_client_setup_blocks_the_launch() {
        let err: std::io::Result<SetupReport> = Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "SGW.exe is locked",
        ));
        let (ok, lines) = setup_status_lines(&err);
        assert!(!ok);
        assert!(lines[0].starts_with("Not launching") && lines[0].contains("locked"));

        let done: std::io::Result<SetupReport> = Ok(SetupReport {
            login_servers_written: true,
            aslr: AslrOutcome::Disabled,
        });
        let (ok, lines) = setup_status_lines(&done);
        assert!(ok);
        assert_eq!(lines.len(), 2);
    }
    use crate::client_paths::WipeReport;
    use crate::worker::Event;

    #[test]
    fn human_bytes_formats_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(2048), "2.00 KB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.00 MB");
    }

    // Drives the same drain-on-overflow shape that `push_status` uses, with
    // a Vec we can inspect directly. Keeps the test free of the full
    // LauncherApp construction (which requires a tokio runtime).
    fn push_capped(buf: &mut Vec<String>, line: String) {
        buf.push(line);
        if buf.len() > MAX_STATUS_LINES {
            let overflow = buf.len() - MAX_STATUS_LINES;
            buf.drain(0..overflow);
        }
    }

    #[test]
    fn push_status_caps_at_max_lines() {
        let mut buf = Vec::new();
        for i in 0..(MAX_STATUS_LINES + 25) {
            push_capped(&mut buf, format!("line {i}"));
        }
        assert_eq!(buf.len(), MAX_STATUS_LINES);
        // Oldest 25 should have been dropped.
        assert_eq!(buf.first().unwrap(), "line 25");
        assert_eq!(
            buf.last().unwrap(),
            &format!("line {}", MAX_STATUS_LINES + 24)
        );
    }

    #[test]
    fn push_status_under_cap_does_not_drain() {
        let mut buf = Vec::new();
        for i in 0..10 {
            push_capped(&mut buf, format!("{i}"));
        }
        assert_eq!(buf.len(), 10);
        assert_eq!(buf.first().unwrap(), "0");
    }

    // Empty install dir: no SGW.exe + no marker → the Install panel
    // should NOT surface the Adopt affordance (nothing to adopt).
    #[test]
    fn should_show_adopt_button_false_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!should_show_adopt_button(dir.path()));
    }

    // SGW.exe present + no marker → adopt is the user's least-destructive
    // path forward. This is the trigger condition.
    #[test]
    fn should_show_adopt_button_true_when_unmanaged_install_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
        assert!(should_show_adopt_button(dir.path()));
    }

    // Marker file already present → install is launcher-managed; adopt
    // is a no-op (and would refuse with AlreadyManaged anyway). Hiding
    // the button keeps the UI honest.
    #[test]
    fn should_show_adopt_button_false_when_already_managed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
        std::fs::write(
            crate::state::InstalledState::path(dir.path()),
            r#"{"applied_patches":[],"seed_sha256":"h"}"#,
        )
        .unwrap();
        assert!(!should_show_adopt_button(dir.path()));
    }

    // status_line_for covers every Event variant that produces a
    // status entry. Wiped is the only one with non-trivial formatting
    // (bytes-freed → human_bytes) — pin its exact shape against a
    // realistic report.
    #[test]
    fn status_line_for_formats_adopt_complete() {
        let line = status_line_for(&Event::AdoptComplete).unwrap();
        assert!(line.contains("Adopted"), "got: {line}");
        assert!(
            line.contains("not verified"),
            "must surface the trust trade-off, got: {line}"
        );
    }

    #[test]
    fn status_line_for_formats_client_patches_note() {
        let line =
            status_line_for(&Event::ClientPatchesNote("off (launcher setting).".into())).unwrap();
        assert_eq!(line, "Client patches: off (launcher setting).");
    }

    #[test]
    fn status_line_for_formats_adopt_error() {
        let line = status_line_for(&Event::AdoptError("boom".into())).unwrap();
        assert_eq!(line, "Adopt failed: boom");
    }

    #[test]
    fn status_line_for_formats_wiped_with_human_bytes() {
        let line = status_line_for(&Event::Wiped {
            kind: "Cache.en-US".into(),
            report: WipeReport {
                entries_removed: 3,
                bytes_freed: 5 * 1024 * 1024,
            },
        })
        .unwrap();
        // Pin both the item count and the human-bytes rendering so a
        // future change to either thread shows up as a test diff.
        assert_eq!(line, "Wiped Cache.en-US: 3 item(s), 5.00 MB freed");
    }

    #[test]
    fn status_line_for_formats_wipe_error() {
        let line = status_line_for(&Event::WipeError("permission denied".into())).unwrap();
        assert_eq!(line, "Wipe failed: permission denied");
    }

    #[test]
    fn status_line_for_returns_none_for_progress_and_manifest_events() {
        // These drive UI state directly (progress bars, manifest
        // summary panel) — they don't belong in the scrolling status
        // log. Returning None enforces that at the type level.
        assert!(status_line_for(&Event::ManifestError("x".into())).is_none());
        assert!(
            status_line_for(&Event::Progress(crate::install::Progress::Downloading {
                label: "seed".into(),
                downloaded: 0,
                total: 0,
            },))
            .is_none()
        );
    }
}
