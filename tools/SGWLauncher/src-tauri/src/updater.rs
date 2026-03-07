use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
pub struct ManifestEntry {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub patch_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchManifest {
    pub version: String,
    pub files: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub updates_available: bool,
    pub files_to_update: Vec<ManifestEntry>,
    pub total_bytes: u64,
    pub server_version: String,
}

pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn check_manifest(
    manifest: &PatchManifest,
    install_path: &Path,
) -> Vec<ManifestEntry> {
    manifest
        .files
        .iter()
        .filter(|entry| {
            let local_path = install_path.join(&entry.path);
            if !local_path.exists() {
                return true;
            }
            match hash_file(&local_path) {
                Ok(hash) => hash != entry.sha256,
                Err(_) => true,
            }
        })
        .cloned()
        .collect()
}

pub async fn fetch_manifest(patch_server_url: &str) -> Result<PatchManifest, UpdateError> {
    let url = format!("{}/manifest.json", patch_server_url.trim_end_matches('/'));
    let manifest: PatchManifest = reqwest::get(&url).await?.json().await?;
    Ok(manifest)
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
        std::fs::write(&path, "hello world").unwrap();
        let hash = hash_file(&path).unwrap();
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_check_manifest_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = PatchManifest {
            version: "1.0".to_string(),
            files: vec![ManifestEntry {
                path: "missing.dat".to_string(),
                sha256: "abc123".to_string(),
                size: 100,
                patch_url: "https://example.com/missing.dat.zip".to_string(),
            }],
        };
        let updates = check_manifest(&manifest, dir.path());
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].path, "missing.dat");
    }

    #[test]
    fn test_check_manifest_matching_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("data.txt");
        std::fs::write(&file_path, "hello world").unwrap();
        let manifest = PatchManifest {
            version: "1.0".to_string(),
            files: vec![ManifestEntry {
                path: "data.txt".to_string(),
                sha256: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
                    .to_string(),
                size: 11,
                patch_url: "https://example.com/data.txt.zip".to_string(),
            }],
        };
        let updates = check_manifest(&manifest, dir.path());
        assert_eq!(updates.len(), 0);
    }

    #[test]
    fn test_check_manifest_changed_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("data.txt");
        std::fs::write(&file_path, "old content").unwrap();
        let manifest = PatchManifest {
            version: "1.0".to_string(),
            files: vec![ManifestEntry {
                path: "data.txt".to_string(),
                sha256: "newsha256hash".to_string(),
                size: 50,
                patch_url: "https://example.com/data.txt.zip".to_string(),
            }],
        };
        let updates = check_manifest(&manifest, dir.path());
        assert_eq!(updates.len(), 1);
    }

    #[test]
    fn test_verify_hash_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();
        let result = verify_hash(
            &path,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        );
        assert!(result.is_ok());
    }
}
