//! The preflight dependency events (C-PD5, D-PD12): one start and one outcome
//! per dependency install, on `launcher.install`.
//!
//! Every field is a closed snake_case code or an integer (R5): no paths, no
//! installer output, no error text.
//!
//! The preflight events (`preflight_checked`, `preflight_refused`) go on the
//! target of the operation they guard (C-PD5).

use super::{CheckId, CheckStatus, PreflightOperation, Report};
use crate::failure::FailureCode;

/// The most `CheckId` codes `missing` names.
const MISSING_MAX: usize = 8;

/// One event on the target of the operation a preflight guards.
macro_rules! on_operation_target {
    ($operation:expr, $level:ident, $($rest:tt)*) => {
        match $operation {
            PreflightOperation::Play => tracing::$level!(target: "launcher.launch", $($rest)*),
            PreflightOperation::Install => tracing::$level!(target: "launcher.install", $($rest)*),
            PreflightOperation::Repair => tracing::$level!(target: "launcher.repair", $($rest)*),
        }
    };
}

/// A preflight finished: INFO when `ready`, WARN otherwise. `blocking_checks`
/// counts the checks that block; `warning_checks` the other checks that are not
/// Ok; `missing` joins the codes of every check that is not Ok (at most 8) and is
/// omitted when all are Ok.
pub fn preflight_checked(report: &Report) {
    let operation = operation_code(report.operation);
    let ready = report.ready;
    let not_ok: Vec<_> = report
        .checks
        .iter()
        .filter(|check| check.status != CheckStatus::Ok)
        .collect();
    let blocking_checks = count(not_ok.iter().filter(|check| check.blocking).count());
    let warning_checks = count(not_ok.len()).saturating_sub(blocking_checks);
    let missing = (!not_ok.is_empty()).then(|| {
        not_ok
            .iter()
            .take(MISSING_MAX)
            .map(|check| check_code(check.id))
            .collect::<Vec<_>>()
            .join(",")
    });
    let missing = missing.as_deref();
    if ready {
        on_operation_target!(
            report.operation,
            info,
            event = "preflight_checked",
            operation,
            ready,
            blocking_checks,
            warning_checks,
            missing,
            "launcher preflight: checks finished; nothing blocks the operation; it may start"
        );
    } else {
        on_operation_target!(
            report.operation,
            warn,
            event = "preflight_checked",
            operation,
            ready,
            blocking_checks,
            warning_checks,
            missing,
            "launcher preflight: checks finished; a required check failed; the player sees what is missing and the fix"
        );
    }
}

/// A dependency the launcher installs for the player (owner D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dependency {
    Rosetta,
    WineRuntime,
    Webview2,
}

impl Dependency {
    /// The closed `dependency` field value.
    pub fn code(self) -> &'static str {
        match self {
            Self::Rosetta => "rosetta",
            Self::WineRuntime => "wine_runtime",
            Self::Webview2 => "webview2",
        }
    }

    /// The closed `source` field value: where the dependency comes from.
    pub fn source(self) -> &'static str {
        match self {
            Self::Rosetta => "apple_software_update",
            Self::WineRuntime => "wowsilicon_release",
            Self::Webview2 => "microsoft_evergreen_bootstrapper",
        }
    }
}

/// How a dependency install ended, as the `outcome` field reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyOutcome {
    Installed,
    Failed,
    Declined,
    TimedOut,
}

impl DependencyOutcome {
    /// The closed `outcome` field value.
    pub fn code(self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Failed => "failed",
            Self::Declined => "declined",
            Self::TimedOut => "timed_out",
        }
    }
}

/// `dependency_install_started` (INFO, `launcher.install`). `download_bytes`
/// is omitted when the size is not known.
pub fn dependency_install_started(dependency: Dependency, download_bytes: Option<u64>) {
    tracing::info!(
        target: "launcher.install",
        event = "dependency_install_started",
        dependency = dependency.code(),
        source = dependency.source(),
        download_bytes,
        "launcher install: a dependency install started; it should finish with the dependency installed; the launcher checks it again afterwards"
    );
}

/// `dependency_install_finished`: INFO when installed, WARN otherwise.
/// `failure_code` is omitted when installed and `exit_code` when the installer
/// gave no exit status.
pub fn dependency_install_finished(
    dependency: Dependency,
    outcome: DependencyOutcome,
    failure_code: Option<FailureCode>,
    elapsed_ms: u64,
    exit_code: Option<i32>,
) {
    let failure_code = failure_code.map(FailureCode::code);
    if outcome == DependencyOutcome::Installed {
        tracing::info!(
            target: "launcher.install",
            event = "dependency_install_finished",
            dependency = dependency.code(),
            outcome = outcome.code(),
            elapsed_ms,
            exit_code,
            "launcher install: a dependency install finished and the re-check found it; nothing else is needed"
        );
    } else {
        tracing::warn!(
            target: "launcher.install",
            event = "dependency_install_finished",
            dependency = dependency.code(),
            outcome = outcome.code(),
            failure_code,
            elapsed_ms,
            exit_code,
            "launcher install: a dependency install did not finish; the dependency was expected afterwards; the player sees the manual fix"
        );
    }
}

/// `preflight_refused` (WARN, the guarded operation's target): the request
/// returned an error, so the player got no report. `check` is omitted for
/// Inspect; `reason` is the `JobError` code, `failure_code` its C-PD1 code.
pub fn preflight_refused(
    operation: PreflightOperation,
    command: &'static str,
    check: Option<CheckId>,
    reason: &'static str,
    failure: FailureCode,
) {
    let operation_name = operation_code(operation);
    let check = check.map(check_code);
    let failure_code = failure.code();
    on_operation_target!(
        operation,
        warn,
        event = "preflight_refused",
        operation = operation_name,
        command,
        check,
        reason,
        failure_code,
        "launcher preflight: the request was refused; expected the checks to run; the player sees the reason and can try again"
    );
}

/// The wire code of an operation, as it serializes.
pub(crate) fn operation_code(operation: PreflightOperation) -> &'static str {
    match operation {
        PreflightOperation::Play => "play",
        PreflightOperation::Install => "install",
        PreflightOperation::Repair => "repair",
    }
}

/// The wire code of a check, as it serializes.
pub(crate) fn check_code(id: CheckId) -> &'static str {
    match id {
        CheckId::Rosetta => "rosetta",
        CheckId::WineRuntime => "wine_runtime",
        CheckId::CompatibilityHelper => "compatibility_helper",
        CheckId::DiskSpace => "disk_space",
        CheckId::GameFolder => "game_folder",
        CheckId::ReleaseManifest => "release_manifest",
        CheckId::AppLocation => "app_location",
        CheckId::Physx => "physx",
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
