use super::*;

/// Native-selected, already downloaded artifact paths. Hashes always come from
/// the authenticated release, never from these paths or from a legacy ledger.
pub struct Artifacts {
    pub seed: PathBuf,
    pub patches: Vec<PathBuf>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Matched,
    KnownTransform,
    Modified,
    Missing,
    Extra,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Difference {
    pub path: String,
    pub source_path: Option<String>,
    pub classification: Classification,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub preview_handle: Uuid,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub release_digest: [u8; 32],
    pub imported_identity: migration::LegacyIdentity,
    pub requested_config: migration::LegacyConfig,
    pub files: Vec<Difference>,
    /// No game-local user data is migrated by policy version 1.
    pub user_data_remains_in_source: bool,
    pub game_telemetry_available: bool,
}
/// Closed choices bound to the native-held report. No renderer paths/config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choices {
    pub normalize_managed_files: bool,
    pub accept_unavailable_game_telemetry: bool,
    pub old_game_closed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Storage(StorageError),
    Operation(ContractError),
    SourceChanged,
    ConsentRequired,
    NoReusableFiles,
    UnsupportedCatalog,
    UnsupportedArchive,
    UnsupportedConfiguration,
    InvalidArtifact,
    Cancelled,
    LauncherTooOld,
}
impl From<StorageError> for Error {
    fn from(e: StorageError) -> Self {
        Self::Storage(e)
    }
}
impl From<ContractError> for Error {
    fn from(e: ContractError) -> Self {
        Self::Operation(e)
    }
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::Storage(StorageError::Io)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub work_id: Uuid,
    pub legacy_install_id: Uuid,
    pub import_digest: String,
    pub reference_digest: [u8; 32],
    pub setup_policy_version: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub installation: InstallIntent,
    pub provenance: Provenance,
    pub imported: migration::LegacyImport,
    pub before: Preferences,
    pub after: Preferences,
    pub report: Report,
    pub choices: Choices,
    pub source_digest: [u8; 32],
    pub stage_identity: inventory::Identity,
    pub destination_identity: inventory::Identity,
}
impl Plan {
    pub fn stage(&self) -> PathBuf {
        self.installation
            .destination
            .join(format!(".cimmeria-adopt-{}", self.provenance.work_id))
    }
    pub(super) fn digest(&self) -> Result<[u8; 32], Error> {
        digest(self)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Staged,
    Promoted,
    Published,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub plan: Plan,
    pub phase: Phase,
}

pub(super) fn digest(value: &impl Serialize) -> Result<[u8; 32], Error> {
    Ok(Sha256::digest(serde_json::to_vec(value).map_err(|_| StorageError::Corrupt)?).into())
}
