//! Worker message types — the [`Command`]s the UI dispatches and the
//! [`Event`]s the worker emits back, plus the launch-time telemetry
//! config the app pre-fetches and hands across the thread boundary.

use std::path::PathBuf;

use crate::client_paths::WipeReport;
use crate::config::LauncherConfig;
use crate::install::Progress;
use crate::manifest::Manifest;
use crate::telemetry::runner::SessionOutcome;

#[derive(Debug, Clone)]
pub enum Command {
    Install {
        config: LauncherConfig,
        manifest: Manifest,
    },
    /// Mark a pre-existing game install as managed by the launcher
    /// without re-downloading the seed. See [`adopt_existing_install`].
    ///
    /// [`adopt_existing_install`]: crate::install::adopt_existing_install
    AdoptExisting {
        install_dir: PathBuf,
        manifest: Manifest,
    },
    LaunchSgw(PathBuf),
    LaunchAteraDebug(PathBuf),
    LaunchAteraFixAslr(PathBuf),
    /// Launch SGW.exe with the `cimmeria-client-telemetry.dll`
    /// side-loaded via the launcher's own injector (issue #417).
    /// `install_dir` is where SGW.exe lives; `dll_path` is the
    /// absolute path to the DLL alongside `sgw-launcher.exe`.
    /// The launcher resolves both paths and hands them through to
    /// [`launch_sgw_with_telemetry`].
    ///
    /// `allow(dead_code)`: Phase 1 of issue #417 lands the worker
    /// dispatch + injector primitives. UI exposure (the "Launch
    /// with telemetry" button + DLL path resolution) is intentionally
    /// deferred to the first hook-set PR so the foundation can land
    /// in isolation. The dispatch routing is covered by
    /// [`tests::launch_sgw_with_client_telemetry_routes_through_dispatch`].
    ///
    /// [`launch_sgw_with_telemetry`]: crate::launch::launch_sgw_with_telemetry
    #[allow(dead_code)]
    LaunchSgwWithClientTelemetry {
        install_dir: PathBuf,
        dll_path: PathBuf,
    },
    /// Launch the Atera debug bat AND run the telemetry pipeline for
    /// the lifetime of the spawned game process. The telemetry config
    /// carries the auth handshake inputs (install_id, machine_id,
    /// branch, git_sha) plus the cimmeria-server URL.
    LaunchAteraDebugWithTelemetry {
        install_dir: PathBuf,
        telemetry: LaunchTelemetryConfig,
    },
    UploadLogs {
        install_dir: PathBuf,
        sas_url: String,
        ledger_path: PathBuf,
    },
    /// Wipe `Documents\My Games\Firesky\SGWGame\Cache.en-US\` — the
    /// server-pushed PAK override cache. Safe to run any time the user
    /// wants the launcher-managed PAKs to win over previously-cached
    /// server pushes.
    WipeClientCache,
    /// Wipe the full `Documents\My Games\Firesky\` tree — the recovery
    /// recipe for cache corruption per docs/client-tools.md. Higher
    /// blast radius (also nukes per-user settings, keybinds, screenshots
    /// if any) — UI must confirm-dialog gate this.
    WipeAllClientState,
    Cancel,
}

#[derive(Debug, Clone)]
pub enum Event {
    ManifestFetched(Manifest),
    ManifestError(String),
    Progress(Progress),
    InstallComplete,
    InstallError(String),
    AdoptComplete,
    AdoptError(String),
    /// Reports per-second visible feedback after the wipe finishes —
    /// `kind` is "Cache.en-US" vs "Firesky" so the UI can render the
    /// right caption.
    Wiped {
        kind: String,
        report: WipeReport,
    },
    WipeError(String),
    Launched(String, u32),
    LaunchError(String),
    /// Telemetry session ended cleanly with a final bundle upload.
    /// Surfaces in the status log so the dev can confirm the upload
    /// completed.
    TelemetrySessionComplete(SessionOutcome),
    /// Telemetry session aborted before bundle upload — auth failed,
    /// chunk POST kept 5xx-ing, etc. The game launched and ran
    /// fine; only the telemetry side died. Streamable to the status
    /// log without blocking on the user.
    TelemetrySessionError(String),
    UploadStarted,
    UploadSkipped(String),
    UploadComplete {
        blob: String,
        bytes: usize,
    },
    UploadError(String),
}

/// Everything the worker needs to bootstrap a telemetry session at
/// game-launch time. Pre-fetched by the app from the identity file +
/// config so the worker thread doesn't need to touch
/// `LauncherIdentity::load_or_mint` itself.
#[derive(Debug, Clone)]
pub struct LaunchTelemetryConfig {
    pub auth_base_url: String,
    pub install_id: String,
    pub machine_id: String,
    pub branch: String,
    pub git_sha: String,
    pub launcher_version: String,
    pub state_dir: PathBuf,
    pub tags: Vec<String>,
}
