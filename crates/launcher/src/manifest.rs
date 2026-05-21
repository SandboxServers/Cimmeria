//! Manifest fetch + validation.
//!
//! The manifest lives at `manifest_url` (defaults to a public Azure Blob URL).
//! Schema:
//!
//! ```json
//! {
//!   "schema": 1,
//!   "seed":    { "blob": "seed/sgw-X.Y.zip", "size": 1234, "sha256": "..." },
//!   "patches": [
//!     { "id": "001-base",    "blob": "patches/001.zip", "size": 123, "sha256": "...", "after": null },
//!     { "id": "002-mercury", "blob": "patches/002.zip", "size": 456, "sha256": "...", "after": "001-base" }
//!   ]
//! }
//! ```
//!
//! Patches are applied in declared order. `after` is validated: every
//! referenced patch id must have been declared earlier in the array.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Unsupported manifest schema {0} (this launcher understands schema 1)")]
    UnsupportedSchema(u32),
    #[error("Manifest patch graph is broken: {0}")]
    BrokenChain(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub seed: SeedEntry,
    #[serde(default)]
    pub patches: Vec<PatchEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedEntry {
    pub blob: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchEntry {
    pub id: String,
    pub blob: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default)]
    pub after: Option<String>,
}

impl Manifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema != 1 {
            return Err(ManifestError::UnsupportedSchema(self.schema));
        }
        let mut seen = std::collections::HashSet::new();
        for p in &self.patches {
            if let Some(after) = &p.after {
                if !seen.contains(after.as_str()) {
                    return Err(ManifestError::BrokenChain(format!(
                        "patch '{}' declares after='{}' which has not been declared yet",
                        p.id, after
                    )));
                }
            }
            if !seen.insert(p.id.as_str()) {
                return Err(ManifestError::BrokenChain(format!(
                    "duplicate patch id '{}'",
                    p.id
                )));
            }
        }
        Ok(())
    }
}

pub async fn fetch_manifest(url: &str) -> Result<Manifest, ManifestError> {
    let body = reqwest::get(url).await?.error_for_status()?.bytes().await?;
    let manifest: Manifest = serde_json::from_slice(&body)?;
    manifest.validate()?;
    Ok(manifest)
}

/// Resolves a blob path (e.g. `seed/sgw.zip`) against the manifest URL's
/// container by stripping the final path segment off the manifest URL.
pub fn blob_url(manifest_url: &str, blob_path: &str) -> String {
    let base = match manifest_url.rsplit_once('/') {
        Some((b, _)) => b,
        None => manifest_url,
    };
    format!("{base}/{blob_path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> SeedEntry {
        SeedEntry {
            blob: "s".into(),
            size: 0,
            sha256: "x".into(),
        }
    }
    fn patch(id: &str, after: Option<&str>) -> PatchEntry {
        PatchEntry {
            id: id.into(),
            blob: format!("p/{id}.zip"),
            size: 1,
            sha256: format!("h-{id}"),
            after: after.map(|s| s.to_string()),
        }
    }

    #[test]
    fn validates_simple_chain() {
        let m = Manifest {
            schema: 1,
            seed: seed(),
            patches: vec![patch("a", None), patch("b", Some("a"))],
        };
        m.validate().unwrap();
    }

    #[test]
    fn rejects_forward_after_ref() {
        let m = Manifest {
            schema: 1,
            seed: seed(),
            patches: vec![patch("a", Some("b")), patch("b", None)],
        };
        assert!(matches!(m.validate(), Err(ManifestError::BrokenChain(_))));
    }

    #[test]
    fn rejects_unknown_after_ref() {
        let m = Manifest {
            schema: 1,
            seed: seed(),
            patches: vec![patch("a", Some("zzz"))],
        };
        assert!(matches!(m.validate(), Err(ManifestError::BrokenChain(_))));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let m = Manifest {
            schema: 1,
            seed: seed(),
            patches: vec![patch("a", None), patch("a", Some("a"))],
        };
        assert!(matches!(m.validate(), Err(ManifestError::BrokenChain(_))));
    }

    #[test]
    fn rejects_unsupported_schema() {
        let m = Manifest {
            schema: 99,
            seed: seed(),
            patches: vec![],
        };
        assert!(matches!(
            m.validate(),
            Err(ManifestError::UnsupportedSchema(99))
        ));
    }

    #[test]
    fn blob_url_strips_manifest_filename() {
        let u = blob_url(
            "https://x.blob.core.windows.net/sgw/manifest.json",
            "seed/s.zip",
        );
        assert_eq!(u, "https://x.blob.core.windows.net/sgw/seed/s.zip");
    }

    #[test]
    fn parses_minimal_manifest_json() {
        let text = r#"{"schema":1,"seed":{"blob":"s","size":1,"sha256":"h"},"patches":[]}"#;
        let m: Manifest = serde_json::from_str(text).unwrap();
        m.validate().unwrap();
        assert_eq!(m.seed.blob, "s");
    }
}
