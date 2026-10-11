//! Native launcher operation ownership, verified content preparation and platform adapters.
mod operations;
pub use operations::*;

mod storage;
pub use storage::{
    AdmissionRequest, DesktopState, ExtractionBackend, HelperPhase, HelperRecord, HelperResult,
    InstallAdmission, InstallIntent, InstalledContent, IntentError, Preferences, StorageError,
};
mod commands;
pub use commands::{NativeCommand, NativeSnapshot};

// Share the existing Windows manifest schema and signature policy verbatim.
// The desktop catalog adds bounded transport; it never calls the legacy fetcher.
pub mod catalog;
pub use cimmeria_launcher_core::manifest;

// One implementation of the legacy installation algorithms on both shells,
// in `cimmeria-launcher-core` (LX-01).
pub use cimmeria_launcher_core::client_setup;
pub use cimmeria_launcher_core::install;
pub use cimmeria_launcher_core::install_layout;
pub use cimmeria_launcher_core::install_report;
pub use cimmeria_launcher_core::patch_dest;
pub use cimmeria_launcher_core::state;
pub use cimmeria_launcher_core::unpack;

pub use cimmeria_launcher_core::install_progress;

// Game telemetry reuses the Windows launcher's session marker and its rule for
// which server addresses may be sent to; the mint itself is bounded here.
pub use cimmeria_launcher_core::telemetry_endpoint;
pub use cimmeria_launcher_core::telemetry_session;

pub mod archive_worker;

pub mod helper_supervisor;

pub use storage::install_recovery;
pub use storage::install_worker;
pub use storage::repair;
pub use storage::runtime_setup;
pub use storage::uninstall;
pub use storage::{EvidenceError, ReleaseIdentity};

#[cfg(target_os = "macos")]
pub mod mac_runtime;

#[cfg(target_os = "macos")]
pub mod mac_wine;

/// Authenticated prerequisite package inputs; execution is a separate operation.
pub mod prerequisites;

pub use storage::launch;
pub use storage::launcher_summary;

pub mod launcher_compatibility;
pub use storage::migration;

pub use storage::adoption;
pub use storage::effective_settings;
pub use storage::game_telemetry;
pub use storage::update;
pub use storage::updater;
mod owner_lock;
