//! Bounded, readable projection of a native-held preview. The complete report
//! stays native and is written to the adoption plan at confirmation.
use super::*;
use cimmeria_launcher_engine::adoption::{Classification, Difference, Preview};
use serde::Serialize;

/// Paths listed for review. Exact matches are counted, not listed.
const LISTED: usize = 60;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub matched: usize,
    pub known_transform: usize,
    pub modified: usize,
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
    pub source: PathBuf,
    pub destination: PathBuf,
    pub release: SignedRelease,
    pub counts: Counts,
    pub differences: Vec<Difference>,
    pub differences_omitted: usize,
    /// Requested order, applied to the copy.
    pub login_servers: Vec<LoginServer>,
    pub client_patches_enabled: bool,
    pub game_telemetry_opted_in: bool,
    pub game_telemetry_available: bool,
    pub requires_normalization: bool,
    pub requires_telemetry_acceptance: bool,
    pub user_data_remains_in_source: bool,
}
impl Review {
    pub(super) fn of(
        preview: &Preview,
        operation_revision: u64,
        preferences_revision: u64,
    ) -> Self {
        let report = preview.report();
        let mut counts = Counts::default();
        let mut differences = Vec::new();
        let mut differences_omitted = 0;
        for file in &report.files {
            match file.classification {
                Classification::Matched => {
                    counts.matched += 1;
                    continue;
                }
                Classification::KnownTransform => counts.known_transform += 1,
                Classification::Modified => counts.modified += 1,
                Classification::Missing => counts.missing += 1,
                Classification::Extra => counts.extra += 1,
            }
            if differences.len() < LISTED {
                differences.push(file.clone());
            } else {
                differences_omitted += 1;
            }
        }
        let manifest = preview.signed_release().manifest();
        Self {
            preview_handle: report.preview_handle,
            operation_revision,
            preferences_revision,
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
            login_servers: report
                .requested_config
                .login_servers
                .iter()
                .map(|server| LoginServer {
                    name: server.name.clone(),
                    url: server.url.clone(),
                })
                .collect(),
            client_patches_enabled: report.requested_config.client_patches.enabled,
            game_telemetry_opted_in: report.requested_config.telemetry.opted_in,
            game_telemetry_available: report.game_telemetry_available,
            requires_normalization: preview.requires_normalization(),
            requires_telemetry_acceptance: preview.requires_telemetry_acceptance(),
            user_data_remains_in_source: report.user_data_remains_in_source,
        }
    }
}
