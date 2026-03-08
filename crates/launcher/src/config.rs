use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

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
    pub server_address: String,
    pub patch_server_url: String,
    pub last_patch_check: Option<String>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            install_path: String::new(),
            server_address: "play.cimmeria.gg".to_string(),
            patch_server_url: "https://patches.cimmeria.gg".to_string(),
            last_patch_check: None,
        }
    }
}

impl LauncherConfig {
    pub fn load(path: &PathBuf) -> Result<Self, ConfigError> {
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), ConfigError> {
        let data = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LauncherConfig::default();
        assert_eq!(config.server_address, "play.cimmeria.gg");
        assert_eq!(config.patch_server_url, "https://patches.cimmeria.gg");
        assert!(config.install_path.is_empty());
        assert!(config.last_patch_check.is_none());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = LauncherConfig {
            install_path: "C:\\Games\\SGW".to_string(),
            server_address: "localhost".to_string(),
            patch_server_url: "https://example.com".to_string(),
            last_patch_check: Some("2026-03-06T12:00:00Z".to_string()),
        };

        config.save(&path).unwrap();
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(loaded.install_path, "C:\\Games\\SGW");
        assert_eq!(loaded.server_address, "localhost");
    }

    #[test]
    fn test_load_missing_file() {
        let path = PathBuf::from("/tmp/nonexistent_sgw_test/config.json");
        assert!(LauncherConfig::load(&path).is_err());
    }
}
