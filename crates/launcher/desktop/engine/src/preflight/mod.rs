//! Preflight (C-PD4): what an operation needs before it starts, as a closed report.
//!
//! The host gathers [`Facts`] (probes, the folder, the disk, the manifest) and
//! [`evaluate`] turns them into a [`Report`] without touching the system, so the
//! blocking rules (D-PD7, D-PD8) are tested on every platform.

use serde::{Deserialize, Serialize};

use crate::failure::FailureCode;

mod disk;
pub mod mac;
pub use disk::{folder_facts, free_bytes};

/// The operation a preflight guards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightOperation {
    Play,
    Install,
    Repair,
}

/// One preflight check; the report lists checks in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckId {
    Rosetta,
    WineRuntime,
    CompatibilityHelper,
    DiskSpace,
    GameFolder,
    ReleaseManifest,
    AppLocation,
    Physx,
}

/// What a check found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Ok,
    Missing,
    Warning,
    Unknown,
}

/// How a check that is not Ok gets fixed (owner D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckAction {
    None,
    AutoInstall,
    Download,
    AskUser,
    Manual,
}

/// One check's result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Check {
    pub id: CheckId,
    pub status: CheckStatus,
    pub action: CheckAction,
    pub failure: Option<FailureCode>,
    pub blocking: bool,
    pub bytes_needed: Option<u64>,
    pub bytes_free: Option<u64>,
    pub download_bytes: Option<u64>,
}

/// The preflight result for one operation; `ready` is true when nothing blocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub operation: PreflightOperation,
    pub checks: Vec<Check>,
    pub ready: bool,
}

/// The outcome of the Rosetta 2 probe (D-PD8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosettaProbe {
    Present,
    Missing,
    TimedOut,
    Error,
}

/// The outcome of fetching the signed release manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestProbe {
    Ok,
    Unreachable,
    Invalid,
    SigningKeyUnavailable,
}

/// What the launcher knows about the saved game folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FolderFacts {
    pub chosen: bool,
    pub exists: bool,
    pub writable: bool,
}

/// Everything [`evaluate`] needs. `None` means the check does not apply and is
/// left out of the report.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// macOS with the Wine backend only.
    pub rosetta: Option<RosettaProbe>,
    /// macOS with the Wine backend only.
    pub wine_runtime_cached: Option<bool>,
    pub helper_available: Option<bool>,
    /// Free bytes at the game folder, or at its nearest existing ancestor.
    pub free_bytes: Option<u64>,
    pub folder: Option<FolderFacts>,
    /// Install and Repair only.
    pub manifest: Option<ManifestProbe>,
    /// macOS only.
    pub translocated: Option<bool>,
    /// Windows only.
    pub physx_installed: Option<bool>,
}

/// Bytes an install or repair needs free (launcher-guide.md, macos.md).
pub const INSTALL_BYTES: u64 = 14_000_000_000;
/// Below this many free bytes, Play warns.
pub const PLAY_WARN_BYTES: u64 = 1_000_000_000;
/// The compatibility runtime download (`mac_runtime` SIZE).
pub const WINE_RUNTIME_BYTES: u64 = 63_744_856;

/// The report schema version.
const SCHEMA_VERSION: u32 = 1;

/// Evaluates the facts for an operation. Pure: no I/O, no clock.
pub fn evaluate(operation: PreflightOperation, facts: &Facts) -> Report {
    let checks: Vec<Check> = [
        facts.rosetta.map(rosetta),
        facts
            .wine_runtime_cached
            .map(|cached| wine_runtime(operation, cached)),
        facts.helper_available.map(helper),
        facts.free_bytes.map(|free| disk_space(operation, free)),
        facts.folder.map(|folder| game_folder(operation, folder)),
        facts.manifest.map(manifest),
        facts.translocated.map(app_location),
        facts.physx_installed.map(physx),
    ]
    .into_iter()
    .flatten()
    .collect();
    let ready = !checks.iter().any(|check| check.blocking);
    Report {
        schema_version: SCHEMA_VERSION,
        operation,
        checks,
        ready,
    }
}

fn ok(id: CheckId) -> Check {
    Check {
        id,
        status: CheckStatus::Ok,
        action: CheckAction::None,
        failure: None,
        blocking: false,
        bytes_needed: None,
        bytes_free: None,
        download_bytes: None,
    }
}

fn not_ok(
    id: CheckId,
    status: CheckStatus,
    action: CheckAction,
    failure: FailureCode,
    blocking: bool,
) -> Check {
    Check {
        status,
        action,
        failure: Some(failure),
        blocking,
        ..ok(id)
    }
}

fn rosetta(probe: RosettaProbe) -> Check {
    use CheckAction::*;
    use CheckStatus::*;
    match probe {
        RosettaProbe::Present => ok(CheckId::Rosetta),
        RosettaProbe::Missing => not_ok(
            CheckId::Rosetta,
            Missing,
            AutoInstall,
            FailureCode::RosettaMissing,
            true,
        ),
        RosettaProbe::TimedOut | RosettaProbe::Error => not_ok(
            CheckId::Rosetta,
            Unknown,
            AskUser,
            FailureCode::RosettaCheckTimedOut,
            false,
        ),
    }
}

fn wine_runtime(operation: PreflightOperation, cached: bool) -> Check {
    if cached {
        return ok(CheckId::WineRuntime);
    }
    // The install downloads the runtime itself, so only Play and Repair wait on it.
    let blocking = operation != PreflightOperation::Install;
    Check {
        download_bytes: Some(WINE_RUNTIME_BYTES),
        ..not_ok(
            CheckId::WineRuntime,
            CheckStatus::Missing,
            CheckAction::Download,
            FailureCode::WineRuntimeMissing,
            blocking,
        )
    }
}

fn helper(available: bool) -> Check {
    if available {
        return ok(CheckId::CompatibilityHelper);
    }
    not_ok(
        CheckId::CompatibilityHelper,
        CheckStatus::Missing,
        CheckAction::Manual,
        FailureCode::HelperUnavailable,
        true,
    )
}

fn disk_space(operation: PreflightOperation, free: u64) -> Check {
    let (needed, status, blocking) = match operation {
        PreflightOperation::Install | PreflightOperation::Repair => {
            (INSTALL_BYTES, CheckStatus::Missing, true)
        }
        PreflightOperation::Play => (PLAY_WARN_BYTES, CheckStatus::Warning, false),
    };
    if free >= needed {
        return ok(CheckId::DiskSpace);
    }
    Check {
        bytes_needed: Some(needed),
        bytes_free: Some(free),
        ..not_ok(
            CheckId::DiskSpace,
            status,
            CheckAction::Manual,
            FailureCode::DiskSpaceLow,
            blocking,
        )
    }
}

fn game_folder(operation: PreflightOperation, folder: FolderFacts) -> Check {
    let missing = |action, failure| {
        not_ok(
            CheckId::GameFolder,
            CheckStatus::Missing,
            action,
            failure,
            true,
        )
    };
    if !folder.chosen {
        return missing(CheckAction::AskUser, FailureCode::GameFolderNotChosen);
    }
    if operation != PreflightOperation::Install && !folder.exists {
        return missing(CheckAction::Manual, FailureCode::GameFolderMissing);
    }
    if !folder.writable {
        return missing(CheckAction::Manual, FailureCode::GameFolderNotWritable);
    }
    ok(CheckId::GameFolder)
}

fn manifest(probe: ManifestProbe) -> Check {
    let failure = match probe {
        ManifestProbe::Ok => return ok(CheckId::ReleaseManifest),
        ManifestProbe::Unreachable => FailureCode::ManifestUnreachable,
        ManifestProbe::Invalid => FailureCode::ManifestInvalid,
        ManifestProbe::SigningKeyUnavailable => FailureCode::SigningKeyUnavailable,
    };
    not_ok(
        CheckId::ReleaseManifest,
        CheckStatus::Missing,
        CheckAction::Manual,
        failure,
        true,
    )
}

fn app_location(translocated: bool) -> Check {
    if !translocated {
        return ok(CheckId::AppLocation);
    }
    not_ok(
        CheckId::AppLocation,
        CheckStatus::Warning,
        CheckAction::Manual,
        FailureCode::AppTranslocated,
        false,
    )
}

fn physx(installed: bool) -> Check {
    if installed {
        return ok(CheckId::Physx);
    }
    // The existing PhysX panel owns the install, so this only warns.
    not_ok(
        CheckId::Physx,
        CheckStatus::Warning,
        CheckAction::AutoInstall,
        FailureCode::PhysxMissing,
        false,
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
