//! Installs the legacy PhysX redistributable into the Proton prefix (PP-08, D-PP7).
//!
//! The prerequisite worker runs the PhysX MSI inside the prefix with
//! `proton runinprefix` and then probes the SDK. Only a `probed` result whose SDK
//! initialized and released sets the stamp to `Installed`. Every other outcome
//! stamps `Failed` and stops the Play: a failed probe is never taken as success.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use cimmeria_runtime_probe::physx::SdkResult;
use cimmeria_runtime_probe::prerequisite::{Failure, PrepareRequest, ResultKind};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::discovery::ProtonInstall;
use super::invocation;
use super::paths;
use super::prefix::{PhysxState, Prefix, ProtonError, Stamp};
use crate::helper_supervisor::{Deadlines, Fault, HelperCommand};
use crate::prerequisites::supervisor::{self, Outcome};

/// The PhysX installer, relative to the game root (the folder above `Working/Binaries`).
const PACKAGE: &str = ".cimmeria-prerequisites/PhysX/PhysX_7.11.13_SystemSoftware.exe";

/// How long the prerequisite worker may run before it is killed.
const DEADLINE: Duration = Duration::from_secs(180);

/// A failed attempt: the closed `proton_physx` result and reason, never a path.
struct Logged {
    result: &'static str,
    reason: String,
}

impl Logged {
    fn new(result: &'static str, reason: &str) -> Self {
        Self {
            result,
            reason: reason.to_owned(),
        }
    }
}

/// Installs PhysX into `prefix` unless its stamp already says `Installed`, then
/// writes the stamp. `helper` is the prerequisite worker, `game_binaries` the
/// host `Working/Binaries` folder and `operation` the Play's id. `log` receives
/// the worker's stderr, appended, when given.
pub async fn ensure_physx(
    install: &ProtonInstall,
    prefix: &Prefix,
    env: &BTreeMap<OsString, OsString>,
    helper: &Path,
    game_binaries: &Path,
    operation: Uuid,
    log: Option<&Path>,
) -> Result<(), ProtonError> {
    if prefix
        .stamp()
        .is_some_and(|stamp| stamp.physx == PhysxState::Installed)
    {
        return Ok(());
    }
    let attempt = run_attempt(install, prefix, env, helper, game_binaries, operation, log).await;
    match attempt {
        Ok(()) => {
            log_result("probed", "none");
            record(prefix, install, PhysxState::Installed)
        }
        Err(logged) => {
            log_result(logged.result, &logged.reason);
            record(prefix, install, PhysxState::Failed)?;
            Err(ProtonError::Setup("physx"))
        }
    }
}

/// Runs the prerequisite worker once and classifies its result. `Ok` only for a
/// `probed` result whose SDK initialized and released.
async fn run_attempt(
    install: &ProtonInstall,
    prefix: &Prefix,
    env: &BTreeMap<OsString, OsString>,
    helper: &Path,
    game_binaries: &Path,
    operation: Uuid,
    log: Option<&Path>,
) -> Result<(), Logged> {
    let game_root = game_binaries
        .parent()
        .and_then(Path::parent)
        .ok_or(Logged::new("failed", "game_root"))?;
    let package = cimmeria_patchset::resolve_existing_case(game_root, Path::new(PACKAGE));
    let scratch = prepare_scratch(prefix, operation)?;
    let request = PrepareRequest {
        schema_version: 1,
        operation_id: operation,
        prefix_generation: Uuid::new_v4(),
        game_binaries: guest(game_binaries)?.into(),
        package: guest(&package)?.into(),
        scratch: guest(&scratch)?.into(),
    };
    let inv = invocation::runinprefix(install, &guest(helper)?, &[]);
    let spec = HelperCommand {
        executable: inv.program,
        arguments: inv.args,
        directory: game_binaries.to_path_buf(),
        environment: env.clone(),
        stderr_log: log.map(PathBuf::from),
    };
    let limits = Deadlines {
        operation: DEADLINE,
        ..Deadlines::default()
    };
    let outcome =
        supervisor::run(spec, request, CancellationToken::new(), limits, |_| Ok(())).await;
    match outcome {
        Outcome::Observed(result) => match result.result {
            ResultKind::Probed { report }
                if report.physx_sdk == (SdkResult::InitializedAndReleased {}) =>
            {
                Ok(())
            }
            ResultKind::Probed { .. } => Err(Logged::new("probed", "sdk_not_initialized")),
            ResultKind::InstallerFailed { installer_code } => Err(Logged {
                result: "installer_failed",
                reason: installer_code.to_string(),
            }),
            ResultKind::Failed { reason } => Err(Logged::new("failed", failure_name(&reason))),
        },
        Outcome::NotStarted(fault) | Outcome::ReconciliationRequired(fault) => {
            Err(Logged::new("failed", fault_name(&fault)))
        }
    }
}

/// A fresh `<state_root>/proton/scratch/<operation>` folder for the worker. The
/// prefix's `compat` folder sits in `<state_root>/proton`.
fn prepare_scratch(prefix: &Prefix, operation: Uuid) -> Result<PathBuf, Logged> {
    let proton = prefix
        .compat
        .parent()
        .ok_or(Logged::new("failed", "scratch_unavailable"))?;
    let scratch = proton.join("scratch");
    std::fs::create_dir_all(&scratch).map_err(|_| Logged::new("failed", "scratch_unavailable"))?;
    let folder = scratch.join(operation.to_string());
    // `create_dir` refuses an existing folder: the worker never overlays an attempt.
    std::fs::create_dir(&folder).map_err(|_| Logged::new("failed", "scratch_unavailable"))?;
    Ok(folder)
}

/// A Windows guest path for the worker request, or a `failed` reason.
fn guest(path: &Path) -> Result<String, Logged> {
    paths::guest(path).map_err(|_| Logged::new("failed", "guest_path"))
}

/// Writes the PhysX state into the stamp, keeping the Proton version it names.
fn record(prefix: &Prefix, install: &ProtonInstall, physx: PhysxState) -> Result<(), ProtonError> {
    let mut stamp = prefix.stamp().unwrap_or(Stamp {
        schema_version: 1,
        proton_version: install.version.clone(),
        physx: PhysxState::NotInstalled,
    });
    stamp.physx = physx;
    prefix.write_stamp(&stamp)
}

/// Logs one `proton_physx` row on `launcher.proton`. The fields are closed names.
fn log_result(result: &str, reason: &str) {
    if result == "probed" {
        tracing::info!(target: "launcher.proton", event = "proton_physx", result, reason);
    } else {
        tracing::warn!(target: "launcher.proton", event = "proton_physx", result, reason);
    }
}

/// The closed snake_case name of a prerequisite failure, for the log.
fn failure_name(failure: &Failure) -> &'static str {
    match failure {
        Failure::InvalidInput => "invalid_input",
        Failure::PackageIdentity => "package_identity",
        Failure::ScratchUnavailable => "scratch_unavailable",
        Failure::Io => "io",
        Failure::Probe => "probe",
    }
}

/// The closed snake_case name of a helper fault, for the log.
fn fault_name(fault: &Fault) -> &'static str {
    match fault {
        Fault::InvalidRequest => "invalid_request",
        Fault::Spawn => "spawn_failed",
        Fault::Ownership => "ownership_failed",
        Fault::Transport => "transport_failed",
        Fault::Protocol => "protocol_failed",
        Fault::Deadline => "deadline",
        Fault::Cancelled => "cancelled",
        Fault::Exit => "exit_failed",
    }
}

#[cfg(test)]
#[path = "physx_tests.rs"]
mod tests;
