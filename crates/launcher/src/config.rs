use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default Azure Blob URL for the seed + patch manifest. Anonymous public read.
pub const DEFAULT_MANIFEST_URL: &str =
    "https://cimmeriastorage.blob.core.windows.net/sgw/manifest.json";

/// Default hostname patched into SGW.exe's `.rdata` (replaces
/// `www.stargateworlds.com`).
pub const DEFAULT_SERVER_HOST: &str = "play.cimmeria.gg";

/// SAS URL for log uploads (PUT-only, scoped to the `logs/` prefix), baked
/// in at compile time from the `LAUNCHER_LOG_SAS_URL` env var. In release CI
/// this is injected from the repo secret; local dev builds without the env
/// var get `None` and the log-upload button is disabled with a friendly note.
pub const LOG_UPLOAD_SAS_URL: Option<&str> = option_env!("LAUNCHER_LOG_SAS_URL");

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub install_path: String,
    pub server_host: String,
    pub manifest_url: String,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            install_path: default_install_path(),
            server_host: DEFAULT_SERVER_HOST.to_string(),
            manifest_url: DEFAULT_MANIFEST_URL.to_string(),
        }
    }
}

impl LauncherConfig {
    pub fn load(path: &PathBuf) -> Result<Self, ConfigError> {
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
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

fn default_install_path() -> String {
    if let Ok(pf) = std::env::var("ProgramFiles") {
        format!("{pf}\\Stargate Worlds")
    } else {
        "C:\\Games\\Stargate Worlds".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        let cfg = LauncherConfig {
            install_path: "X".into(),
            server_host: "Y".into(),
            manifest_url: "Z".into(),
        };
        cfg.save(&path).unwrap();
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(loaded.install_path, "X");
        assert_eq!(loaded.server_host, "Y");
        assert_eq!(loaded.manifest_url, "Z");
    }

    #[test]
    fn load_missing_file_errors() {
        assert!(LauncherConfig::load(&PathBuf::from("/no/such/path.json")).is_err());
    }

    #[test]
    fn default_uses_program_files_when_set() {
        let prev = std::env::var("ProgramFiles").ok();
        std::env::set_var("ProgramFiles", "Z:\\PF");
        let cfg = LauncherConfig::default();
        assert_eq!(cfg.install_path, "Z:\\PF\\Stargate Worlds");
        match prev {
            Some(v) => std::env::set_var("ProgramFiles", v),
            None => std::env::remove_var("ProgramFiles"),
        }
    }
}
