use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Native-selected folders. Explicit mapping permits Windows paths imported on Mac.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacySource {
    pub launcher_directory: PathBuf,
    pub game_directory: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyIdentity {
    pub schema_version: u32,
    pub install_id: Uuid,
    pub machine_id: String,
    pub first_seen_ms: i64,
    pub created_by_launcher_version: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginServer {
    pub name: String,
    pub url: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyConfig {
    #[serde(default = "one")]
    pub schema_version: u32,
    pub install_path: PathBuf,
    pub manifest_url: String,
    #[serde(default = "servers")]
    pub login_servers: Vec<LoginServer>,
    #[serde(default)]
    pub telemetry: LegacyTelemetry,
    #[serde(default)]
    pub client_patches: LegacyPatches,
}
fn one() -> u32 {
    1
}
fn yes() -> bool {
    true
}
fn servers() -> Vec<LoginServer> {
    vec![LoginServer {
        name: "Cimmeria".into(),
        url: "http://play.cimmeria.app:8081".into(),
    }]
}
fn auth_url() -> String {
    "http://play.cimmeria.app:8081/api".into()
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyTelemetry {
    // Historical `enabled` deliberately has no alias: it never proved consent.
    #[serde(default)]
    pub opted_in: bool,
    #[serde(default)]
    pub prompt_answered: bool,
    #[serde(default = "auth_url")]
    pub auth_url: String,
}
impl Default for LegacyTelemetry {
    fn default() -> Self {
        Self {
            opted_in: false,
            prompt_answered: false,
            auth_url: auth_url(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyPatches {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub dll_override: Option<PathBuf>,
}
impl Default for LegacyPatches {
    fn default() -> Self {
        Self {
            enabled: true,
            dll_override: None,
        }
    }
}
/// Historical claims only. Never an ownership, readiness or deletion receipt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyLedger {
    #[serde(default)]
    pub applied_patches: Vec<String>,
    #[serde(default)]
    pub seed_sha256: Option<String>,
    #[serde(default)]
    pub seed_adopted: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyImport {
    pub source: LegacySource,
    pub identity: LegacyIdentity,
    pub config: LegacyConfig,
    pub ledger: LegacyLedger,
    /// SHA-256 over source paths and exact length-delimited file bytes.
    pub confirmation: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationError {
    Storage(super::StorageError),
    MissingSource,
    UnsupportedSchema,
    InvalidSource,
    SourceChanged,
    Conflict,
    Busy,
}
impl From<super::StorageError> for MigrationError {
    fn from(value: super::StorageError) -> Self {
        Self::Storage(value)
    }
}
