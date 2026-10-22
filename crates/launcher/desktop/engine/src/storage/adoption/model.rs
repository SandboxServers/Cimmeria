use super::*;
use cimmeria_launcher_core::{file_index::FileClass, state::IncompletePatch};

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
    /// In place, with no stock seed: patches replay tolerantly over the folder's
    /// own files and the published index is only compared (D-LA9, D-LA14).
    Light,
}
/// How the adopted files were checked. A full check verifies every file against
/// the release; a light check never blocks on a changed file. Old plans carry no
/// field and read as `Full`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileCheck {
    #[default]
    Full,
    Light,
}
impl FileCheck {
    pub(super) fn is_full(&self) -> bool {
        *self == Self::Full
    }
}
/// One op a light adoption skipped, for the review. The path is the target's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LightSkip {
    pub path: String,
    /// The patch's state key.
    pub patch: String,
    /// A closed skip code (`SkipReason::code`).
    pub reason: String,
}
/// What a light preview found, beside the `Report` (which the frontend decodes
/// strictly, R8). Lists are capped at [`LIGHT_LIST_CAP`]; the totals are not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LightSummary {
    pub patches_total: u32,
    pub patches_current: u32,
    pub patches_pending: u32,
    pub download_bytes: u64,
    pub skipped: Vec<LightSkip>,
    pub skipped_total: u32,
    pub index_state: super::index_compare::IndexState,
    pub index_counts: std::collections::BTreeMap<FileClass, u64>,
    pub index_files: Vec<(String, FileClass)>,
    pub index_files_total: u32,
}
/// The longest list a [`LightSummary`] carries.
pub const LIGHT_LIST_CAP: usize = 500;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Why the signed file index was not used (C15). Not serialized: the frontend
    /// decoder rejects unknown properties (R8); the reason goes to the log and telemetry.
    #[serde(skip)]
    pub index_fallback: Option<super::index_source::IndexFallback>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download: Option<DownloadEstimate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup: Option<PathBuf>,
    /// The folder's `LoginInternal.lua` differs from its release and prepared bytes.
    /// Its original is copied to the backup before setup rewrites it (D-AI13).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub login_replaced: bool,
}
/// Plan identity. `index_fallback` is informational and is never serialized, so a
/// reloaded record cannot carry it; it is left out so the stored plan still matches.
/// Every other field is destructured here, so a new field must be compared explicitly.
impl PartialEq for Report {
    fn eq(&self, other: &Self) -> bool {
        let Report {
            preview_handle,
            source,
            destination,
            release_digest,
            imported_identity,
            requested_config,
            files,
            user_data_remains_in_source,
            game_telemetry_available,
            index_fallback: _,
            download,
            backup,
            login_replaced,
        } = self;
        *preview_handle == other.preview_handle
            && *source == other.source
            && *destination == other.destination
            && *release_digest == other.release_digest
            && *imported_identity == other.imported_identity
            && *requested_config == other.requested_config
            && *files == other.files
            && *user_data_remains_in_source == other.user_data_remains_in_source
            && *game_telemetry_available == other.game_telemetry_available
            && *download == other.download
            && *backup == other.backup
            && *login_replaced == other.login_replaced
    }
}
impl Eq for Report {}
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// The folder holds entries that cannot be adopted safely; the report names
    /// up to five of them by relative path.
    Unsafe(Box<UnsafeReport>),
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

/// Why one entry of a refused folder cannot be adopted (launcher-adopt-ux D-AU4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsafeReason {
    /// A symbolic link, or on Windows a junction.
    Link,
    /// A OneDrive or other cloud-files placeholder (Windows reparse tag).
    CloudFile,
    /// A file with more than one name (hard link count above 1).
    HardLink,
    /// Not a plain file or folder: another reparse point, a socket, a device.
    SpecialFile,
    /// A name the file index cannot hold: not ASCII, over 4096 bytes, or with `:` or `\`.
    Name,
    /// Another entry has the same name apart from letter case.
    CaseCollision,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsafeEntry {
    pub reason: UnsafeReason,
    /// Relative to the chosen folder, `/`-separated, at most 200 chars plus `…`.
    /// `"."` is the folder itself; `None` means the folder's own location is not
    /// its canonical path. Never absolute.
    pub path: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsafeReport {
    /// At most `MAX_UNSAFE_SHOWN` (5), in walk order (children sorted by name).
    pub entries: Vec<UnsafeEntry>,
    /// Every offender the survey found, listed or not.
    pub total: u32,
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
    /// Absent in a plan an earlier build wrote, which read as a full check.
    #[serde(default, skip_serializing_if = "FileCheck::is_full")]
    pub check: FileCheck,
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
    /// Patches a light adoption applied with skipped ops; they are also in the
    /// ledger's `applied_patches` (D-LA18). Empty for every other adoption.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incomplete_patches: Vec<IncompletePatch>,
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
            incomplete_patches,
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
            #[serde(skip_serializing_if = "Vec::is_empty")]
            incomplete_patches: &'a Vec<IncompletePatch>,
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
            incomplete_patches,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> Report {
        Report {
            preview_handle: Uuid::nil(),
            source: PathBuf::from("source"),
            destination: PathBuf::from("destination"),
            release_digest: [7; 32],
            imported_identity: None,
            requested_config: None,
            files: Vec::new(),
            user_data_remains_in_source: true,
            game_telemetry_available: false,
            index_fallback: None,
            download: None,
            backup: None,
            login_replaced: false,
        }
    }

    #[test]
    fn index_fallback_is_not_plan_identity() {
        let mut reloaded = report();
        reloaded.index_fallback = Some(super::super::index_source::IndexFallback {
            reason: "index_not_published",
            check: None,
            http_status: None,
            seed_bytes: 42,
        });
        assert_eq!(report(), reloaded);
    }

    #[test]
    fn other_fields_are_plan_identity() {
        let mut changed = report();
        changed.login_replaced = true;
        assert_ne!(report(), changed);
        let mut moved = report();
        moved.destination = PathBuf::from("elsewhere");
        assert_ne!(report(), moved);
    }
}
