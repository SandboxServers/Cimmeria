//! "Changes to your client": what the launcher set up, patched, injected and
//! what the server pushes. Read-only. The rows and their wording come from
//! `cimmeria_launcher_core::client_changes`; this module supplies the inputs
//! from this install's verified state.
use super::*;
use cimmeria_launcher_core::client_changes as core_changes;
#[cfg(test)]
mod tests;

/// A `client_changes_command` request from the webview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientChangesCommand {
    /// List the changes.
    Inspect { schema_version: u32 },
}

/// Path-free and safe for IPC: descriptions name client-relative files only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientChangesStatus {
    pub schema_version: u32,
    /// False when there is no verified installation; `changes` is then empty.
    pub installed: bool,
    /// Grouped in [`ChangeGroup`] order: setup, patches, launch, server.
    pub changes: Vec<ClientChangeRow>,
    /// The copy's record of applied patches exists but could not be read, so
    /// every patch row reads as pending. Repair rewrites it.
    pub install_record_unreadable: bool,
}

/// Which launch DLLs this launcher build carries, verified. The shell decides
/// from its bundle; Play drops a DLL the policy wants when the bundle lacks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundledDlls {
    pub client_patches: bool,
    pub client_telemetry: bool,
}

impl BundledDlls {
    /// None when the build has no verified bundle at all.
    pub fn of(resources: Option<&launch::Resources>) -> Self {
        Self {
            client_patches: resources.is_some_and(|r| r.client_patches.is_some()),
            client_telemetry: resources.is_some_and(|r| r.client_telemetry.is_some()),
        }
    }
}

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientChangeRow {
    pub group: ChangeGroup,
    /// The group's heading, so the webview does not keep its own copy.
    pub heading: String,
    pub title: String,
    pub description: String,
    pub status: ChangeStatus,
    pub status_label: String,
    /// The manifest id, for patch rows.
    pub patch_id: Option<String>,
}

/// IPC mirror of `core_changes::ChangeGroup`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeGroup {
    Setup,
    Patch,
    Launch,
    Server,
}

/// IPC mirror of `core_changes::ChangeStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    EveryLaunch,
    AtInstall,
    Applied,
    Pending,
    OnLaunch,
    Off,
    DuringPlay,
    Missing,
}

// Exhaustive on purpose: a new core variant must get an IPC name here.
impl From<core_changes::ChangeGroup> for ChangeGroup {
    fn from(value: core_changes::ChangeGroup) -> Self {
        match value {
            core_changes::ChangeGroup::Setup => Self::Setup,
            core_changes::ChangeGroup::Patch => Self::Patch,
            core_changes::ChangeGroup::Launch => Self::Launch,
            core_changes::ChangeGroup::Server => Self::Server,
        }
    }
}

impl From<core_changes::ChangeStatus> for ChangeStatus {
    fn from(value: core_changes::ChangeStatus) -> Self {
        match value {
            core_changes::ChangeStatus::EveryLaunch => Self::EveryLaunch,
            core_changes::ChangeStatus::AtInstall => Self::AtInstall,
            core_changes::ChangeStatus::Applied => Self::Applied,
            core_changes::ChangeStatus::Pending => Self::Pending,
            core_changes::ChangeStatus::OnLaunch => Self::OnLaunch,
            core_changes::ChangeStatus::Off => Self::Off,
            core_changes::ChangeStatus::DuringPlay => Self::DuringPlay,
            core_changes::ChangeStatus::Missing => Self::Missing,
        }
    }
}

impl From<core_changes::ClientChange> for ClientChangeRow {
    fn from(change: core_changes::ClientChange) -> Self {
        Self {
            group: change.group.into(),
            heading: change.group.heading().into(),
            title: change.title,
            description: change.description,
            status: change.status.into(),
            status_label: change.status.label().into(),
            patch_id: change.patch_id,
        }
    }
}

/// Serializes as one flat code string, like `StorageError`: `"unavailable"`,
/// or the storage code itself (`"io"`, `"unsupported_schema"`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientChangesError {
    /// This build cannot do it yet.
    Unavailable,
    /// State-directory failure, including an unknown schema.
    #[serde(untagged)]
    Storage(StorageError),
}
impl From<StorageError> for ClientChangesError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl DesktopState {
    /// Reads only, so it takes no mutation admission. Anything added here
    /// that writes must call `ensure_no_active_operation` first.
    /// `bundled` is what this build carries, so the launch rows say what Play
    /// actually loads rather than what the policy alone would ask for.
    pub fn client_changes(
        &self,
        request: ClientChangesCommand,
        bundled: BundledDlls,
    ) -> Result<ClientChangesStatus, ClientChangesError> {
        let ClientChangesCommand::Inspect { schema_version } = request;
        if schema_version != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        let Some((installed, provenance)) = self.installed_for_launch()? else {
            return Ok(ClientChangesStatus {
                schema_version: 1,
                installed: false,
                changes: Vec::new(),
                install_record_unreadable: false,
            });
        };
        // The patches come from the installed copy's own verified release, not
        // the latest catalog: the list describes this client, not an update.
        // Applied state is the copy's own `launcher-installed.json`, written
        // when its content was prepared; a copy whose content is gone (Repair)
        // has none, and every patch reads as pending. An unreadable one reads
        // the same way, as in the egui launcher, but the list says so.
        let game = installed.intent.destination.join("game");
        let (ledger, install_record_unreadable) = match read::<crate::state::InstalledState>(
            &crate::state::InstalledState::path(&game),
        ) {
            Ok(ledger) => (ledger.unwrap_or_default(), false),
            Err(_) => (crate::state::InstalledState::default(), true),
        };
        let policy = self.injection_policy()?;
        let changes = core_changes::list(&core_changes::ChangeInputs {
            patches: &installed.release.manifest().patches,
            installed: &ledger,
            client_patches_enabled: policy.client_patches,
            telemetry_opted_in: policy.client_telemetry,
            launch: core_changes::LaunchFacts {
                adopted: provenance.is_some(),
                patches_dll_present: bundled.client_patches,
                telemetry_dll_present: bundled.client_telemetry,
            },
        });
        Ok(ClientChangesStatus {
            schema_version: 1,
            installed: true,
            changes: changes.into_iter().map(Into::into).collect(),
            install_record_unreadable,
        })
    }
}
