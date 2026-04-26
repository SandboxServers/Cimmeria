use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Hash mismatch for {file}: expected {expected}, got {actual}")]
    HashMismatch {
        file: String,
        expected: String,
        actual: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchEntry {
    pub id: String,           // stable identifier across republishes
    #[serde(rename = "type")]
    pub patch_type: String,   // "rar" or "zip"
    pub version: String,
    pub sha256: String,       // hash of the download file (zip or rar)
    pub size: u64,
    pub url: String,          // direct download URL
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchManifest {
    pub manifest_version: u32,
    pub published_at: String,
    pub patches: Vec<PatchEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppliedPatches {
    pub applied: HashMap<String, String>, // patch_id -> sha256 of applied zip
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub updates_available: bool,
    pub patches_to_apply: Vec<PatchEntry>,
    pub total_bytes: u64,
    pub manifest_version: u32,
}

pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn check_manifest(
    manifest: &PatchManifest,
    applied: &AppliedPatches,
) -> Vec<PatchEntry> {
    manifest
        .patches
        .iter()
        .filter(|entry| {
            match applied.applied.get(&entry.id) {
                None => true,
                Some(applied_hash) => applied_hash != &entry.sha256,
            }
        })
        .cloned()
        .collect()
}

pub async fn fetch_manifest(manifest_url: &str) -> Result<PatchManifest, UpdateError> {
    let manifest: PatchManifest = reqwest::get(manifest_url).await?.json().await?;
    Ok(manifest)
}

pub fn load_applied_patches(config_dir: &Path) -> AppliedPatches {
    let path = config_dir.join("applied-patches.json");
    if !path.exists() {
        return AppliedPatches::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => AppliedPatches::default(),
    }
}

pub fn save_applied_patches(config_dir: &Path, applied: &AppliedPatches) -> Result<(), UpdateError> {
    if let Some(parent) = config_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(applied)?;
    let path = config_dir.join("applied-patches.json");
    std::fs::write(&path, data)?;
    Ok(())
}

pub fn verify_hash(path: &Path, expected: &str) -> Result<(), UpdateError> {
    let actual = hash_file(path).map_err(UpdateError::Io)?;
    if actual != expected {
        return Err(UpdateError::HashMismatch {
            file: path.display().to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"hello world").unwrap();
        let hash = hash_file(&path).unwrap();
        assert_eq!(hash.len(), 64, "SHA-256 hex must be 64 characters");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_verify_hash_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"hello world").unwrap();
        let hash = hash_file(&path).unwrap();
        let result = verify_hash(&path, &hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_hash_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"hello world").unwrap();
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_hash(&path, wrong_hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_manifest_absent_patch() {
        let manifest = PatchManifest {
            manifest_version: 1,
            published_at: "2026-04-25T00:00:00Z".to_string(),
            patches: vec![PatchEntry {
                id: "patch-001".to_string(),
                patch_type: "zip".to_string(),
                version: "1.0.0".to_string(),
                sha256: "abc123".to_string(),
                size: 1024,
                url: "https://example.com/patch.zip".to_string(),
            }],
        };
        let applied = AppliedPatches::default();
        let to_apply = check_manifest(&manifest, &applied);
        assert_eq!(to_apply.len(), 1);
        assert_eq!(to_apply[0].id, "patch-001");
    }

    #[test]
    fn test_check_manifest_already_applied() {
        let manifest = PatchManifest {
            manifest_version: 1,
            published_at: "2026-04-25T00:00:00Z".to_string(),
            patches: vec![PatchEntry {
                id: "patch-001".to_string(),
                patch_type: "zip".to_string(),
                version: "1.0.0".to_string(),
                sha256: "abc123".to_string(),
                size: 1024,
                url: "https://example.com/patch.zip".to_string(),
            }],
        };
        let mut applied = AppliedPatches::default();
        applied.applied.insert("patch-001".to_string(), "abc123".to_string());
        let to_apply = check_manifest(&manifest, &applied);
        assert_eq!(to_apply.len(), 0);
    }

    #[test]
    fn test_check_manifest_hash_mismatch() {
        let manifest = PatchManifest {
            manifest_version: 1,
            published_at: "2026-04-25T00:00:00Z".to_string(),
            patches: vec![PatchEntry {
                id: "patch-001".to_string(),
                patch_type: "zip".to_string(),
                version: "1.0.0".to_string(),
                sha256: "newsha123".to_string(),
                size: 1024,
                url: "https://example.com/patch.zip".to_string(),
            }],
        };
        let mut applied = AppliedPatches::default();
        applied.applied.insert("patch-001".to_string(), "oldsha123".to_string());
        let to_apply = check_manifest(&manifest, &applied);
        assert_eq!(to_apply.len(), 1, "Patch with hash mismatch should be re-applied");
        assert_eq!(to_apply[0].id, "patch-001");
    }

    #[test]
    fn test_load_applied_patches_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let patches = load_applied_patches(dir.path());
        assert_eq!(patches.applied.len(), 0);
    }

    #[test]
    fn test_save_and_load_applied_patches() {
        let dir = tempfile::tempdir().unwrap();
        let mut patches = AppliedPatches::default();
        patches.applied.insert("patch-001".to_string(), "hash1".to_string());
        patches.applied.insert("patch-002".to_string(), "hash2".to_string());

        save_applied_patches(dir.path(), &patches).unwrap();

        let loaded = load_applied_patches(dir.path());
        assert_eq!(loaded.applied.len(), 2);
        assert_eq!(loaded.applied.get("patch-001").unwrap(), "hash1");
        assert_eq!(loaded.applied.get("patch-002").unwrap(), "hash2");
    }
}
