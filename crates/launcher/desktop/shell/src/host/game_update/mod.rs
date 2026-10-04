//! Native-owned signed game offers. Checking never admits an operation.
use super::*;
use cimmeria_launcher_engine::{catalog::VerifiedRelease, ReleaseIdentity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub(super) mod dispatch;
mod maintenance;
use maintenance::{Maintenance, MaintenanceAction};

#[cfg(test)]
#[derive(Clone)]
pub(super) struct TestDispatch {
    url: String,
    interrupt: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum GameUpdateCommand {
    Maintain {
        schema_version: u32,
        action: MaintenanceAction,
        operation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
    },
    Rollback {
        schema_version: u32,
        completed_update: Uuid,
        operation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
    },
    Cancel {
        schema_version: u32,
        operation_id: Uuid,
    },
    Apply {
        schema_version: u32,
        offer_id: Uuid,
        operation_id: Uuid,
        operation_revision: u64,
        confirmed: bool,
    },
    Inspect {
        schema_version: u32,
    },
    Check {
        schema_version: u32,
        operation_revision: u64,
    },
}
impl GameUpdateCommand {
    pub fn validate(&self) -> Result<(), JobError> {
        let version = match self {
            Self::Inspect { schema_version }
            | Self::Check { schema_version, .. }
            | Self::Apply { schema_version, .. }
            | Self::Cancel { schema_version, .. }
            | Self::Maintain { schema_version, .. }
            | Self::Rollback { schema_version, .. } => *schema_version,
        };
        if version == 1 {
            Ok(())
        } else {
            Err(JobError::UnsupportedSchema)
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Check {
    id: Uuid,
    revision: u64,
    installation: Uuid,
    current: ReleaseIdentity,
    native_backend: bool,
    directory: PathBuf,
}
#[derive(Default)]
pub(super) struct Offers {
    check: Option<Check>,
    release: Option<VerifiedRelease>,
}
#[derive(Debug, Serialize)]
pub struct GameUpdateStatus {
    pub schema_version: u32,
    pub native: NativeSnapshot,
    pub can_check: bool,
    pub checked: bool,
    pub offer: Option<Offer>,
    pub progress: Option<super::install::contract::JobProgress>,
    pub maintenance: Option<Maintenance>,
}
#[derive(Debug, Serialize)]
pub struct Offer {
    pub id: Uuid,
    pub installation_id: Uuid,
    pub directory: PathBuf,
    pub current_digest: String,
    pub target_digest: String,
    pub current_patches: Vec<String>,
    pub target_patches: Vec<String>,
    pub launcher_update_required: bool,
}
fn digest(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn idle(state: &DesktopState) -> bool {
    !state.requires_reopen()
        && state.ensure_updater_idle().is_ok()
        && state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .is_none_or(|op| op.state.terminal())
}
impl NativeHost {
    pub(crate) fn begin_game_update_check(&self, revision: u64) -> Result<Check, JobError> {
        let mut offers = self.game_update_offer.lock().map_err(|_| JobError::Io)?;
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| JobError::Io)?;
        if !idle(&state) {
            return Err(JobError::Busy);
        }
        if state.operations().snapshot().revision != revision {
            return Err(JobError::StaleRevision);
        }
        let installed = state
            .installed_content()?
            .ok_or(JobError::IdentityConflict)?;
        let check = Check {
            id: Uuid::new_v4(),
            revision,
            installation: installed.intent.operation_id,
            current: installed.current_release,
            native_backend: installed.intent.backend.is_native(),
            directory: installed.intent.destination,
        };
        *offers = Offers {
            check: Some(check.clone()),
            release: None,
        };
        Ok(check)
    }
    pub(crate) fn finish_game_update_check(
        &self,
        check: Check,
        release: VerifiedRelease,
    ) -> Result<GameUpdateStatus, JobError> {
        {
            let mut offers = self.game_update_offer.lock().map_err(|_| JobError::Io)?;
            if offers.check.as_ref() != Some(&check) {
                return Err(JobError::StaleRevision);
            }
            let store = self.store()?;
            let mut state = store.lock().map_err(|_| JobError::Io)?;
            if !idle(&state) {
                return Err(JobError::Busy);
            }
            if state.operations().snapshot().revision != check.revision {
                return Err(JobError::StaleRevision);
            }
            let installed = state
                .installed_content()?
                .ok_or(JobError::IdentityConflict)?;
            if installed.intent.operation_id != check.installation
                || installed.current_release != check.current
            {
                return Err(JobError::IdentityConflict);
            }
            offers.release = Some(release);
        }
        self.game_update_status()
    }
    pub fn game_update_status(&self) -> Result<GameUpdateStatus, JobError> {
        let progress = {
            let worker = self.game_update_worker.lock().map_err(|_| JobError::Io)?;
            worker.as_ref().and_then(|worker| {
                worker
                    .preparation
                    .progress
                    .borrow()
                    .as_ref()
                    .map(|value| (worker.id, super::install::progress(value)))
            })
        };
        let offers = self.game_update_offer.lock().map_err(|_| JobError::Io)?;
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| JobError::Io)?;
        let native = state.dispatch(NativeCommand::Inspect { schema_version: 1 })?;
        let mut status = GameUpdateStatus {
            schema_version: 1,
            can_check: false,
            checked: false,
            progress: progress
                .filter(|(id, _)| {
                    native
                        .operation
                        .operation
                        .as_ref()
                        .is_some_and(|op| op.id == *id && !op.state.terminal())
                })
                .map(|(_, progress)| progress),
            offer: None,
            maintenance: maintenance::status(&mut state)?,
            native,
        };
        // Active preparation owns a Windows root lock. Never reopen it just to inspect.
        if !idle(&state) {
            return Ok(status);
        }
        let Some(installed) = state.installed_content()? else {
            return Ok(status);
        };
        status.can_check = true;
        let (Some(check), Some(release)) = (&offers.check, &offers.release) else {
            return Ok(status);
        };
        if check.revision != state.operations().snapshot().revision
            || check.installation != installed.intent.operation_id
            || check.current != installed.current_release
        {
            return Ok(status);
        }
        status.checked = true;
        if release.digest() != installed.current_release.manifest_digest {
            status.offer = Some(Offer {
                id: check.id,
                installation_id: check.installation,
                directory: check.directory.clone(),
                current_digest: digest(check.current.manifest_digest),
                target_digest: digest(release.digest()),
                current_patches: installed
                    .release
                    .manifest()
                    .patches
                    .iter()
                    .map(|patch| patch.id.clone())
                    .collect(),
                target_patches: release
                    .manifest()
                    .patches
                    .iter()
                    .map(|patch| patch.id.clone())
                    .collect(),
                launcher_update_required: state
                    .launcher_compatibility()
                    .for_release(release)
                    .blocks(),
            });
        }
        Ok(status)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;
