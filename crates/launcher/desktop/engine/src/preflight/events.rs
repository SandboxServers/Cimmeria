//! The preflight dependency events (C-PD5, D-PD12): one start and one outcome
//! per dependency install, on `launcher.install`.
//!
//! Every field is a closed snake_case code or an integer (R5): no paths, no
//! installer output, no error text.

use crate::failure::FailureCode;

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
