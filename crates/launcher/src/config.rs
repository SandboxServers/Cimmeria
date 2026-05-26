use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::state::atomic_write;

/// Default manifest URL. Points at the GitHub Release that owns the
/// rolling `content-current` tag — operators publish a new manifest by
/// `--clobber`-uploading `manifest.json` to that tag. See
/// [`docs/client/launcher-distribution-setup.md`](../../docs/client/launcher-distribution-setup.md).
pub const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/SandboxServers/Cimmeria/releases/download/content-current/manifest.json";

/// Default hostname patched into SGW.exe's `.rdata` (replaces
/// `www.stargateworlds.com`).
pub const DEFAULT_SERVER_HOST: &str = "play.cimmeria.gg";

/// SAS URL for log uploads (PUT-only, scoped to the `logs/` prefix), baked
/// in at compile time from the `LAUNCHER_LOG_SAS_URL` env var. In release CI
/// this is injected from the repo secret; local dev builds without the env
/// var get `None` and the log-upload button is disabled with a friendly note.
pub const LOG_UPLOAD_SAS_URL: Option<&str> = option_env!("LAUNCHER_LOG_SAS_URL");

/// Current persisted config-file schema. Bump when a breaking change to
/// the on-disk shape lands; the load path then refuses to deserialise
/// against the wrong schema rather than silently defaulting fields.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error(
        "Config schema version {got} unsupported (expected {expected}). \
        Delete the config file to regenerate with defaults."
    )]
    UnsupportedSchema { got: u32, expected: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    /// Bumped on incompatible changes to this struct's on-disk shape.
    /// Defaults to the current schema on legacy / missing field, which
    /// is safe because adding `serde(default)` fields is always
    /// backwards-compatible by construction.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub install_path: PathBuf,
    pub server_host: String,
    pub manifest_url: String,
}

fn default_schema_version() -> u32 {
    CONFIG_SCHEMA_VERSION
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            install_path: default_install_path(),
            server_host: DEFAULT_SERVER_HOST.to_string(),
            manifest_url: DEFAULT_MANIFEST_URL.to_string(),
        }
    }
}

impl LauncherConfig {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let data = std::fs::read_to_string(path)?;
        let cfg: LauncherConfig = serde_json::from_str(&data)?;
        if cfg.schema_version != CONFIG_SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedSchema {
                got: cfg.schema_version,
                expected: CONFIG_SCHEMA_VERSION,
            });
        }
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let bytes = serde_json::to_vec_pretty(self)?;
        atomic_write(path, &bytes)?;
        Ok(())
    }
}

pub fn config_path() -> PathBuf {
    exe_dir().join("launcher-config.json")
}

pub fn ledger_path() -> PathBuf {
    exe_dir().join("uploaded.json")
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Default install directory. Lives under `%LOCALAPPDATA%` so the launcher
/// can write without UAC elevation — `%ProgramFiles%` requires admin and
/// produces an opaque mid-extract failure for non-admin users.
fn default_install_path() -> PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local).join("Stargate Worlds");
    }
    // Non-Windows fallback used for tests and Linux dev builds.
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/Stargate Worlds");
    }
    PathBuf::from("Stargate Worlds")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        let cfg = LauncherConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            install_path: PathBuf::from("X"),
            server_host: "Y".into(),
            manifest_url: "Z".into(),
        };
        cfg.save(&path).unwrap();
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(loaded.install_path, PathBuf::from("X"));
        assert_eq!(loaded.server_host, "Y");
        assert_eq!(loaded.manifest_url, "Z");
    }

    #[test]
    fn load_missing_file_errors() {
        assert!(LauncherConfig::load(&PathBuf::from("/no/such/path.json")).is_err());
    }

    // Legacy config files written before schema_version existed should
    // load cleanly — `#[serde(default)]` populates the missing field.
    #[test]
    fn load_legacy_without_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"install_path":"X","server_host":"Y","manifest_url":"Z"}"#,
        )
        .unwrap();
        let cfg = LauncherConfig::load(&path).unwrap();
        assert_eq!(cfg.schema_version, CONFIG_SCHEMA_VERSION);
        assert_eq!(cfg.install_path, PathBuf::from("X"));
    }

    // Future-schema configs should be rejected explicitly rather than
    // silently coerced (would otherwise erase fields not in the current
    // struct).
    #[test]
    fn load_unsupported_schema_version_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"schema_version":99,"install_path":"X","server_host":"Y","manifest_url":"Z"}"#,
        )
        .unwrap();
        let err = LauncherConfig::load(&path).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::UnsupportedSchema {
                got: 99,
                expected: 1
            }
        ));
    }

    // Windows-only: the assertion compares against a path built from
    // backslash-separated string literals (`Z:\LocalApp\Stargate Worlds`),
    // which only parse as multi-component paths on Windows. On Linux
    // `PathBuf::from("Z:\\LocalApp")` is a single-component string and
    // `.join("Stargate Worlds")` produces `Z:\LocalApp/Stargate Worlds`
    // — different on the byte level from the expected literal. The
    // production behavior under test (`LOCALAPPDATA` → `<dir>/Stargate
    // Worlds`) is a Windows convention; the cross-platform fallback
    // branches in `default_install_path` are exercised implicitly by
    // every other test that constructs a `LauncherConfig::default()`.
    #[cfg(target_os = "windows")]
    #[test]
    fn default_uses_localappdata_when_set() {
        let prev_local = std::env::var("LOCALAPPDATA").ok();
        std::env::set_var("LOCALAPPDATA", "Z:\\LocalApp");
        let cfg = LauncherConfig::default();
        assert_eq!(
            cfg.install_path,
            PathBuf::from("Z:\\LocalApp\\Stargate Worlds")
        );
        match prev_local {
            Some(v) => std::env::set_var("LOCALAPPDATA", v),
            None => std::env::remove_var("LOCALAPPDATA"),
        }
    }
}
