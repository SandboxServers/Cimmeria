use super::*;

/// Native-selected, already downloaded artifact paths. Hashes always come from
/// the authenticated release, never from these paths or from a legacy ledger.
pub struct Artifacts {
    pub seed: PathBuf,
    pub patches: Vec<PathBuf>,
}
/// Where an adoption's files come from: the saved import of the old launcher,
/// or a folder the player picked natively.
// The legacy variant stays unboxed so the source reads like the saved import.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptionSource {
    Legacy(migration::LegacyImport),
    Folder(FolderSource),
}
/// A borrowed view of the one source a [`Plan`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdoptionSourceRef<'a> {
    Legacy(&'a migration::LegacyImport),
    Folder(&'a FolderSource),
}
/// A folder the player picked natively, reviewed and then pinned by its scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderSource {
    /// What the player picked: the client root, its `Working`, or `Working\Binaries`.
    pub selected: PathBuf,
    /// Canonical client root, the folder holding `Working` (AI-10 resolves it).
    pub root: PathBuf,
    /// `digest(&inventory::scan(root))` at review. Replaces `import_digest` as the
    /// "folder changed after review" guard.
    pub scan_digest: [u8; 32],
}
/// How an adoption treats the player's folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdoptionMode {
    InPlace,
    Copy,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Matched,
    KnownTransform,
    /// A release file the folder holds in a form the release must be patched into.
    NeedsPatching,
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
/// What an adoption would download before it can finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadEstimate {
    pub seed: bool,
    pub patches: Vec<String>,
    pub bytes: u64,
    /// The seed comes from the player's own game archive, not a download.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub local_seed: bool,
}
/// One file the adopted tree gets from the replacement store (AI-09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replacement {
    pub path: String,
    pub size: u64,
    pub sha256: [u8; 32],
    pub reason: Classification,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub preview_handle: Uuid,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub release_digest: [u8; 32],
    /// Absent for a folder source, which has no imported identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_identity: Option<migration::LegacyIdentity>,
    /// Absent for a folder source, which has no imported configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_config: Option<migration::LegacyConfig>,
    pub files: Vec<Difference>,
    /// No game-local user data is migrated by policy version 1.
    pub user_data_remains_in_source: bool,
    pub game_telemetry_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download: Option<DownloadEstimate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup: Option<PathBuf>,
    /// The folder's `LoginInternal.lua` differs from its release and prepared bytes.
    /// Its original is copied to the backup before setup rewrites it (D-AI13).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub login_replaced: bool,
}
impl Report {
    /// Adopted in place (D-LX12): the reviewed folder becomes the owner root.
    /// A verified copy always refuses a destination equal to its source.
    pub fn in_place(&self) -> bool {
        self.source == self.destination
    }
    /// Release files that keep an in-place adoption from verifying: changed,
    /// absent or patch-only bytes that the preview holds no replacement for. A
    /// replacement from the replacement store (AI-09) takes the file's place, and
    /// a file the release keeps at another spelling moves rather than blocks.
    pub fn blocking<'a>(
        &'a self,
        replacements: &'a [Replacement],
    ) -> impl Iterator<Item = &'a Difference> {
        self.files.iter().filter(move |d| {
            self.in_place()
                && match d.classification {
                    Classification::Modified
                    | Classification::Missing
                    | Classification::NeedsPatching => {
                        !replacements.iter().any(|r| r.path == d.path)
                    }
                    Classification::KnownTransform
                    | Classification::Matched
                    | Classification::Extra => false,
                }
        })
    }
}
/// Closed choices bound to the native-held report. No renderer paths/config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choices {
    pub normalize_managed_files: bool,
    pub accept_unavailable_game_telemetry: bool,
    pub old_game_closed: bool,
    /// Folder adoption: replace the files the review marked modified.
    #[serde(default, skip_serializing_if = "is_false")]
    pub replace_modified_files: bool,
}
fn is_false(value: &bool) -> bool {
    !*value
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
    /// The signed blob could not be fetched; nothing was accepted.
    Network,
    Cancelled,
    LauncherTooOld,
    /// An in-place folder holds release files that do not match the signed
    /// release; the review names them.
    VerificationFailed,
    /// In place: the folder is not the full `Working\Binaries` layout the
    /// installer writes, so its release files are not where the release keeps them.
    UnsupportedLayout,
    /// In place: `SGW.exe` in the folder is open, so the game is running.
    GameRunning,
    /// In place: a release file is both in `game` and at its old path, and
    /// neither copy is the one the review verified. The log names it.
    DuplicateReleaseFile,
    /// In place: the folder holds launcher-owned names that are not the
    /// harmless leftovers of an admission that never began.
    LeftoverLauncherFiles,
    /// The folder cannot be written, so the adoption cannot change it.
    FolderNotWritable,
    /// The selected folder is neither a client root nor its `Working` folder.
    NotAGameFolder,
    /// The backup folder for replaced files already exists.
    BackupConflict,
    /// This adoption mode is not available in this build.
    UnsupportedMode,
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
    /// Absent for a folder source, which has no legacy install.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_install_id: Option<Uuid>,
    /// Absent for a folder source; `folder_scan_digest` guards it instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_digest: Option<String>,
    pub reference_digest: [u8; 32],
    pub setup_policy_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_scan_digest: Option<[u8; 32]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_index_sha256: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub installation: InstallIntent,
    pub provenance: Provenance,
    /// Set for a legacy source; exactly one of `imported` and `folder` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported: Option<migration::LegacyImport>,
    /// Set for a folder source; exactly one of `imported` and `folder` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<FolderSource>,
    pub before: Preferences,
    pub after: Preferences,
    pub report: Report,
    pub choices: Choices,
    pub source_digest: [u8; 32],
    pub stage_identity: inventory::Identity,
    pub destination_identity: inventory::Identity,
    /// Files the adopted tree takes from the replacement store; empty for a
    /// legacy adoption.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replacements: Vec<Replacement>,
}
impl Plan {
    pub fn stage(&self) -> PathBuf {
        self.installation
            .destination
            .join(format!(".cimmeria-adopt-{}", self.provenance.work_id))
    }
    /// The one source this plan names. A plan with both or neither set is damaged.
    pub fn source(&self) -> Result<AdoptionSourceRef<'_>, Error> {
        match (&self.imported, &self.folder) {
            (Some(imported), None) => Ok(AdoptionSourceRef::Legacy(imported)),
            (None, Some(folder)) => Ok(AdoptionSourceRef::Folder(folder)),
            _ => Err(StorageError::Corrupt.into()),
        }
    }
    /// The operation's intent digest. Preferences go through `digest_form`,
    /// so a plan a schema-1 build wrote (and journaled) before the upgrade
    /// still hashes to the digest that build recorded.
    pub(super) fn digest(&self) -> Result<[u8; 32], Error> {
        // Destructured so a new Plan field cannot silently fall out of the
        // digest. Field names and order are the serialized Plan's.
        let Plan {
            schema_version,
            installation,
            provenance,
            imported,
            folder,
            before,
            after,
            report,
            choices,
            source_digest,
            stage_identity,
            destination_identity,
            replacements,
        } = self;
        #[derive(Serialize)]
        struct Digested<'a> {
            schema_version: &'a u32,
            installation: &'a InstallIntent,
            provenance: &'a Provenance,
            #[serde(skip_serializing_if = "Option::is_none")]
            imported: Option<&'a migration::LegacyImport>,
            #[serde(skip_serializing_if = "Option::is_none")]
            folder: Option<&'a FolderSource>,
            before: PreferencesDigest<'a>,
            after: PreferencesDigest<'a>,
            report: &'a Report,
            choices: &'a Choices,
            source_digest: &'a [u8; 32],
            stage_identity: &'a inventory::Identity,
            destination_identity: &'a inventory::Identity,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            replacements: &'a Vec<Replacement>,
        }
        digest(&Digested {
            schema_version,
            installation,
            provenance,
            imported: imported.as_ref(),
            folder: folder.as_ref(),
            before: before.digest_form(),
            after: after.digest_form(),
            report,
            choices,
            source_digest,
            stage_identity,
            destination_identity,
            replacements,
        })
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
