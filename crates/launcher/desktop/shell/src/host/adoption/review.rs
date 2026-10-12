//! Bounded, readable projection of a native-held preview. The complete report
//! stays native and is written to the adoption plan at confirmation.
use super::*;
use cimmeria_launcher_engine::adoption::{
    AdoptionSourceRef, Classification, Difference, DownloadEstimate, Preview,
};
use serde::Serialize;

/// Paths listed for review. Exact matches are counted, not listed.
const LISTED: usize = 60;

/// Which kind of source the review was prepared from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Legacy,
    Folder,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub matched: usize,
    pub known_transform: usize,
    pub modified: usize,
    /// Files the folder holds in a form the release must be patched into.
    pub needs_patching: usize,
    pub missing: usize,
    pub extra: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SignedRelease {
    pub manifest_sha256: String,
    pub seed_sha256: String,
    /// Signed patch order.
    pub patches: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoginServer {
    pub name: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Review {
    pub preview_handle: Uuid,
    pub operation_revision: u64,
    pub preferences_revision: u64,
    pub source_kind: SourceKind,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub release: SignedRelease,
    pub counts: Counts,
    pub differences: Vec<Difference>,
    pub differences_omitted: usize,
    /// A modified or missing release file would be replaced; the player must allow it.
    pub requires_replacement: bool,
    /// Where the files a replacement moves out of the way are kept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_directory: Option<PathBuf>,
    /// What the adoption downloads before it can finish, when anything is needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<DownloadEstimate>,
    /// Requested order, applied to the copy. Legacy sources only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_servers: Option<Vec<LoginServer>>,
    /// Legacy sources only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_patches_enabled: Option<bool>,
    /// Legacy sources only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_telemetry_opted_in: Option<bool>,
    pub game_telemetry_available: bool,
    pub requires_normalization: bool,
    pub requires_telemetry_acceptance: bool,
    pub user_data_remains_in_source: bool,
    /// The folder itself is adopted; nothing is copied.
    pub in_place: bool,
    /// Release files that keep the folder from verifying, so it cannot be
    /// adopted in place until they are restored. Bounded like `differences`.
    pub blocked_files: Vec<String>,
}
impl Review {
    pub(super) fn of(
        preview: &Preview,
        operation_revision: u64,
        preferences_revision: u64,
    ) -> Result<Self, AdoptionError> {
        let report = preview.report();
        let source_kind = match preview.source() {
            AdoptionSourceRef::Legacy(_) => SourceKind::Legacy,
            AdoptionSourceRef::Folder(_) => SourceKind::Folder,
        };
        // A legacy preview always carries its requested config; a folder preview has
        // none (no legacy import), so the requested-config fields stay absent for it.
        let requested = match source_kind {
            SourceKind::Legacy => Some(
                report
                    .requested_config
                    .as_ref()
                    .ok_or(AdoptionError::CorruptState)?,
            ),
            SourceKind::Folder => None,
        };
        let mut counts = Counts::default();
        let mut differences = Vec::new();
        let mut differences_omitted = 0;
        let mut requires_replacement = false;
        for file in &report.files {
            match file.classification {
                Classification::Matched => {
                    counts.matched += 1;
                    continue;
                }
                Classification::KnownTransform => counts.known_transform += 1,
                Classification::NeedsPatching => counts.needs_patching += 1,
                Classification::Modified => {
                    requires_replacement |= report.in_place();
                    counts.modified += 1;
                }
                Classification::Missing => {
                    requires_replacement |= report.in_place();
                    counts.missing += 1;
                }
                Classification::Extra => counts.extra += 1,
            }
            if differences.len() < LISTED {
                differences.push(file.clone());
            } else {
                differences_omitted += 1;
            }
        }
        let manifest = preview.signed_release().manifest();
        Ok(Self {
            preview_handle: report.preview_handle,
            operation_revision,
            preferences_revision,
            source_kind,
            source: report.source.clone(),
            destination: report.destination.clone(),
            release: SignedRelease {
                manifest_sha256: report
                    .release_digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect(),
                seed_sha256: manifest.seed.sha256.to_ascii_lowercase(),
                patches: manifest.patches.iter().map(|p| p.id.clone()).collect(),
            },
            counts,
            differences,
            differences_omitted,
            requires_replacement,
            backup_directory: report.backup.clone(),
            download: report.download.clone(),
            login_servers: requested.map(|requested| {
                requested
                    .login_servers
                    .iter()
                    .map(|server| LoginServer {
                        name: server.name.clone(),
                        url: server.url.clone(),
                    })
                    .collect()
            }),
            client_patches_enabled: requested.map(|requested| requested.client_patches.enabled),
            game_telemetry_opted_in: requested.map(|requested| requested.telemetry.opted_in),
            game_telemetry_available: report.game_telemetry_available,
            requires_normalization: preview.requires_normalization(),
            requires_telemetry_acceptance: preview.requires_telemetry_acceptance(),
            user_data_remains_in_source: report.user_data_remains_in_source,
            in_place: report.in_place(),
            blocked_files: report
                .blocking(preview.replacements())
                .take(LISTED)
                .map(|file| file.path.clone())
                .collect(),
        })
    }
}
