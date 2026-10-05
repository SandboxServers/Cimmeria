//! Launcher-summary composition: the product version, the endpoint (none in
//! this build) and the closed codes for a command that failed before admission.
//! Nothing here carries a path, a URL, a name or an error text.
use super::{InstallCommand, JobError, LaunchCommand, NativeHost};
use cimmeria_launcher_engine::{
    launcher_summary::{self, SummaryConfig, SummaryErrorCode, SummaryOperation, SummaryPhase},
    DesktopState,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Called once, right after the state is first opened.
pub(super) fn start(state: &Arc<Mutex<DesktopState>>) {
    launcher_summary::start(state, config());
}

fn config() -> SummaryConfig {
    SummaryConfig {
        launcher_version: version(env!("CARGO_PKG_VERSION")),
        // No endpoint is configured in this build, so nothing is tracked,
        // recorded or sent, whatever the consent preference says.
        endpoint: None,
    }
}

// "a.b.c", with any pre-release or build suffix ignored.
fn version(text: &str) -> (u16, u16, u16) {
    let mut parts = text
        .split(['.', '-', '+'])
        .map(|part| part.parse::<u16>().unwrap_or(0));
    let mut next = || parts.next().unwrap_or(0);
    (next(), next(), next())
}

// Only failures of the journey itself. A stale revision, a busy launcher or an
// unknown operation is a contract answer to the webview, not an attempt's end.
//
// The phase is derived from the error kind, not observed. `CorruptState` and
// `Io` also come from steps that run before admission is tried (a cached
// release lookup, a runtime reconcile, a worker thread that could not be
// joined), so `admission` with `state_invalid` or `local_io` means "before or
// at admission", not that the journal refused the attempt.
fn failure(error: JobError) -> Option<(SummaryPhase, SummaryErrorCode)> {
    Some(match error {
        JobError::PlatformUnavailable => (
            SummaryPhase::PlatformCheck,
            SummaryErrorCode::PlatformUnavailable,
        ),
        JobError::LauncherTooOld => (
            SummaryPhase::CompatibilityCheck,
            SummaryErrorCode::LauncherTooOld,
        ),
        JobError::InvalidDirectory => (
            SummaryPhase::DestinationCheck,
            SummaryErrorCode::InvalidDirectory,
        ),
        JobError::ManifestUnavailable => (
            SummaryPhase::CatalogFetch,
            SummaryErrorCode::ManifestUnavailable,
        ),
        JobError::InvalidManifest => (
            SummaryPhase::ManifestVerify,
            SummaryErrorCode::ManifestInvalid,
        ),
        JobError::SigningKeyUnavailable => (
            SummaryPhase::ManifestVerify,
            SummaryErrorCode::SigningKeyUnavailable,
        ),
        JobError::CorruptState => (SummaryPhase::Admission, SummaryErrorCode::StateInvalid),
        JobError::Io => (SummaryPhase::Admission, SummaryErrorCode::LocalIo),
        JobError::UnsupportedSchema
        | JobError::StaleRevision
        | JobError::Busy
        | JobError::UnknownOperation
        | JobError::IdentityConflict
        | JobError::RecoveryRequired
        | JobError::PersistenceUncertain => return None,
    })
}

impl InstallCommand {
    /// The attempt this command starts, with the operation id the webview
    /// named. The id is only compared with the journal; it is never exported.
    pub fn summary_attempt(&self) -> Option<(SummaryOperation, Option<Uuid>)> {
        let (operation, operation_id) = match self {
            Self::Install { operation_id, .. } => (SummaryOperation::Install, operation_id),
            Self::PrepareRuntime { operation_id, .. } => {
                (SummaryOperation::PrepareRuntime, operation_id)
            }
            Self::Repair { operation_id, .. } => (SummaryOperation::Repair, operation_id),
            Self::Uninstall { operation_id, .. } => (SummaryOperation::Uninstall, operation_id),
            _ => return None,
        };
        Some((operation, Some(*operation_id)))
    }
}

impl LaunchCommand {
    /// As `InstallCommand::summary_attempt`: only Play starts an attempt.
    pub fn summary_attempt(&self) -> Option<(SummaryOperation, Option<Uuid>)> {
        match self {
            Self::Play { operation_id, .. } => {
                Some((SummaryOperation::Launch, Some(*operation_id)))
            }
            Self::Inspect { .. } => None,
        }
    }
}

impl NativeHost {
    /// Records that a command failed before the journal admitted it. An attempt
    /// the journal did admit reports through the journal instead; the engine
    /// tells the two apart by the operation id.
    pub fn note_command_failure(
        &self,
        attempt: Option<(SummaryOperation, Option<Uuid>)>,
        error: JobError,
    ) {
        let (Some((operation, operation_id)), Some((phase, code))) = (attempt, failure(error))
        else {
            return;
        };
        // Never opens the state: a store that is not open has nothing configured.
        let Some(state) = self.state.lock().ok().and_then(|state| state.clone()) else {
            return;
        };
        if let Ok(mut state) = state.lock() {
            state.summary_pre_admission_failure(operation_id, operation, phase, code);
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_versions_parse_to_three_numbers() {
        assert_eq!(version("0.1.0"), (0, 1, 0));
        assert_eq!(version("12.34.56"), (12, 34, 56));
        assert_eq!(version("1.2.3-beta.4"), (1, 2, 3));
        assert_eq!(version("1.2.3+build.9"), (1, 2, 3));
        assert_eq!(version("1.2"), (1, 2, 0));
        assert_eq!(version(""), (0, 0, 0));
        // The crate's own version is a plain three-part one.
        let (major, minor, patch) = version(env!("CARGO_PKG_VERSION"));
        assert_eq!(
            format!("{major}.{minor}.{patch}"),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn only_journey_failures_map_to_a_phase_and_a_code() {
        use SummaryErrorCode as Code;
        use SummaryPhase as Phase;
        let mapped = [
            (
                JobError::PlatformUnavailable,
                Phase::PlatformCheck,
                Code::PlatformUnavailable,
            ),
            (
                JobError::LauncherTooOld,
                Phase::CompatibilityCheck,
                Code::LauncherTooOld,
            ),
            (
                JobError::InvalidDirectory,
                Phase::DestinationCheck,
                Code::InvalidDirectory,
            ),
            (
                JobError::ManifestUnavailable,
                Phase::CatalogFetch,
                Code::ManifestUnavailable,
            ),
            (
                JobError::InvalidManifest,
                Phase::ManifestVerify,
                Code::ManifestInvalid,
            ),
            (
                JobError::SigningKeyUnavailable,
                Phase::ManifestVerify,
                Code::SigningKeyUnavailable,
            ),
            (JobError::CorruptState, Phase::Admission, Code::StateInvalid),
            (JobError::Io, Phase::Admission, Code::LocalIo),
        ];
        for (error, phase, code) in mapped {
            assert_eq!(failure(error), Some((phase, code)), "{error:?}");
        }
        for error in [
            JobError::UnsupportedSchema,
            JobError::StaleRevision,
            JobError::Busy,
            JobError::UnknownOperation,
            JobError::IdentityConflict,
            JobError::RecoveryRequired,
            JobError::PersistenceUncertain,
        ] {
            assert_eq!(failure(error), None, "{error:?}");
        }
    }

    #[test]
    fn only_commands_that_start_an_attempt_name_one() {
        let id = Uuid::new_v4();
        let other = Uuid::new_v4();
        let attempt = |command: InstallCommand| command.summary_attempt();
        assert_eq!(
            attempt(InstallCommand::Install {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                preferences_revision: 0,
            }),
            Some((SummaryOperation::Install, Some(id)))
        );
        assert_eq!(
            attempt(InstallCommand::PrepareRuntime {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                installation_id: other,
            }),
            Some((SummaryOperation::PrepareRuntime, Some(id)))
        );
        assert_eq!(
            attempt(InstallCommand::Repair {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                installation_id: other,
                confirmed: true,
            }),
            Some((SummaryOperation::Repair, Some(id)))
        );
        assert_eq!(
            attempt(InstallCommand::Uninstall {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                installation_id: other,
                confirmed: true,
            }),
            Some((SummaryOperation::Uninstall, Some(id)))
        );
        for command in [
            InstallCommand::Inspect { schema_version: 1 },
            InstallCommand::Cancel {
                schema_version: 1,
                operation_id: id,
            },
            InstallCommand::Resume {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
            },
            InstallCommand::Reconcile {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
            },
            InstallCommand::CleanFailed {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                confirmed: true,
            },
            // Recovery of an admitted repair belongs to that repair's attempt.
            InstallCommand::RecoverRepair {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                confirmed: true,
            },
            InstallCommand::AbandonRepair {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                confirmed: true,
            },
            InstallCommand::CleanupRepair {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                confirmed: true,
            },
        ] {
            assert_eq!(attempt(command), None);
        }
        assert_eq!(
            LaunchCommand::Play {
                schema_version: 1,
                operation_id: id,
                operation_revision: 0,
                installation_id: other,
            }
            .summary_attempt(),
            Some((SummaryOperation::Launch, Some(id)))
        );
        assert_eq!(
            LaunchCommand::Inspect { schema_version: 1 }.summary_attempt(),
            None
        );
    }

    fn queue_file(root: &tempfile::TempDir) -> std::path::PathBuf {
        root.path().join("state").join("launcher-summaries.json")
    }

    fn opt_in(host: &NativeHost) {
        host.dispatch(cimmeria_launcher_engine::NativeCommand::SavePreferences {
            schema_version: 1,
            expected_revision: 0,
            install_directory: None,
            launcher_summary_consent: true,
        })
        .unwrap();
    }

    #[test]
    fn this_build_configures_no_endpoint() {
        assert!(config().endpoint.is_none());
        assert_eq!(
            config().launcher_version,
            version(env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn noting_a_failure_never_opens_the_state_and_changes_nothing_once_open() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let attempt = Some((SummaryOperation::Install, Some(Uuid::new_v4())));
        host.note_command_failure(attempt, JobError::ManifestUnavailable);
        assert!(!root.path().join("state").exists());

        // Open and opted in. This build configures no endpoint, and that alone
        // keeps the failure from being recorded: the positive control below
        // differs only in having one.
        opt_in(&host);
        host.note_command_failure(attempt, JobError::ManifestUnavailable);
        host.note_command_failure(None, JobError::ManifestUnavailable);
        host.note_command_failure(attempt, JobError::Busy);
        assert!(!queue_file(&root).exists());
    }

    #[test]
    fn a_journey_failure_reaches_the_queue_once_an_endpoint_is_configured() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        opt_in(&host);
        // This build has no endpoint; give the open state one that is never
        // contacted. No exporter is started by this call.
        host.with_state(|state| {
            state.configure_summaries(SummaryConfig {
                launcher_version: (0, 1, 0),
                endpoint: launcher_summary::SummaryEndpoint::parse("http://127.0.0.1:9"),
            });
            Ok(())
        })
        .unwrap();
        let entries = || -> Vec<serde_json::Value> {
            let bytes = std::fs::read(queue_file(&root)).unwrap();
            let queue: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            queue["entries"].as_array().unwrap().clone()
        };

        // A contract answer and a command that names no attempt record nothing.
        let install = Some((SummaryOperation::Install, Some(Uuid::new_v4())));
        host.note_command_failure(install, JobError::Busy);
        host.note_command_failure(None, JobError::ManifestUnavailable);
        assert!(!queue_file(&root).exists());

        // The same failure twice is one row with one repeat.
        host.note_command_failure(install, JobError::ManifestUnavailable);
        host.note_command_failure(install, JobError::ManifestUnavailable);
        let queued = entries();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0]["pre_admission"], true);
        let row = &queued[0]["summary"];
        assert_eq!(row["operation"], "install");
        assert_eq!(row["phase"], "catalog_fetch");
        assert_eq!(row["outcome"], "failed");
        assert_eq!(row["error_code"], "manifest_unavailable");
        assert_eq!(row["retry_count"], 1);

        // A launcher too old for the release is refused before the journal, so
        // this glue is its only way to a row.
        let launch = Some((SummaryOperation::Launch, Some(Uuid::new_v4())));
        host.note_command_failure(launch, JobError::LauncherTooOld);
        let queued = entries();
        assert_eq!(queued.len(), 2);
        assert_eq!(queued[1]["pre_admission"], true);
        let row = &queued[1]["summary"];
        assert_eq!(row["operation"], "launch");
        assert_eq!(row["phase"], "compatibility_check");
        assert_eq!(row["outcome"], "failed");
        assert_eq!(row["error_code"], "launcher_too_old");
        assert_eq!(row["retry_count"], 0);

        // Still nothing for a contract answer.
        host.note_command_failure(launch, JobError::Busy);
        host.note_command_failure(install, JobError::Busy);
        assert_eq!(entries(), queued);
    }
}
