//! Manifest fetch + validation + Ed25519 signature verification.
//!
//! The manifest lives at `manifest_url` (defaults to a GitHub Release
//! download URL). Schema:
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
//!
//! Signing model
//! -------------
//! `fetch_manifest` fetches both `<url>` and `<url>.sig`. The signature
//! file is hex-encoded (128 chars = 64 raw bytes) and is verified against
//! the embedded [`MANIFEST_SIGNING_PUBKEY`]. This closes the manifest-
//! tampering hole Cady flagged on PR #343: anyone who could MITM or
//! compromise the manifest host can no longer ship arbitrary seed/patch
//! payloads — SHA verification of the artifacts is meaningful again
//! because the manifest itself is now authenticated.
//!
//! The dev-default public key has a publicly-known private counterpart
//! (`DEV_MANIFEST_PRIVKEY` in tests) so anyone can sign manifests for
//! development. Production builds **must** override via the
//! `LAUNCHER_MANIFEST_PUBKEY_HEX` env var at compile time — the release
//! workflow injects it from a repo secret.

use std::sync::LazyLock;

use ed25519_dalek::{Signature, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 32-byte Ed25519 private key whose public counterpart is the dev
/// default for [`MANIFEST_SIGNING_PUBKEY`]. Only compiled into the test
/// binary; production code never needs the signing half.
#[cfg(test)]
pub(crate) const DEV_MANIFEST_PRIVKEY: [u8; 32] = [0x2a; 32];

/// Embedded Ed25519 public key used to verify the detached
/// `manifest.json.sig` against the fetched `manifest.json` bytes.
///
/// Defaults to the public counterpart of `[0x2a; 32]` so dev / PR / CI
/// builds work out of the box with a publicly-known private key. Set
/// `LAUNCHER_MANIFEST_PUBKEY_HEX` at compile time to override with the
/// production key (64 hex chars = 32 bytes).
pub static MANIFEST_SIGNING_PUBKEY: LazyLock<VerifyingKey> = LazyLock::new(|| {
    if let Some(hex_str) = option_env!("LAUNCHER_MANIFEST_PUBKEY_HEX") {
        let bytes = hex_decode_32(hex_str)
            .expect("LAUNCHER_MANIFEST_PUBKEY_HEX must be 64 hex chars (32 bytes)");
        return VerifyingKey::from_bytes(&bytes)
            .expect("LAUNCHER_MANIFEST_PUBKEY_HEX is not a valid Ed25519 public key");
    }
    // Dev / PR / CI builds — derive the pubkey from the well-known dev
    // private key. A future improvement could `tracing::warn!` here, but
    // it would fire on every test run too which is noisy.
    SigningKey::from_bytes(&[0x2a; 32]).verifying_key()
});

/// Decode exactly 64 hex chars into a `[u8; 32]`. Accepts both upper-
/// and lower-case hex. Used by [`MANIFEST_SIGNING_PUBKEY`] and by
/// `verify_signature`.
fn hex_decode_32(s: &str) -> Result<[u8; 32], &'static str> {
    if s.len() != 64 {
        return Err("expected 64 hex chars");
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        let pair = &s[i * 2..i * 2 + 2];
        *byte = u8::from_str_radix(pair, 16).map_err(|_| "invalid hex")?;
    }
    Ok(out)
}

/// Decode exactly 128 hex chars into a `[u8; 64]` (an Ed25519 signature).
fn hex_decode_64(s: &str) -> Result<[u8; 64], &'static str> {
    let trimmed = s.trim();
    if trimmed.len() != 128 {
        return Err("expected 128 hex chars");
    }
    let mut out = [0u8; 64];
    for (i, byte) in out.iter_mut().enumerate() {
        let pair = &trimmed[i * 2..i * 2 + 2];
        *byte = u8::from_str_radix(pair, 16).map_err(|_| "invalid hex")?;
    }
    Ok(out)
}

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
    #[error("Refusing to fetch manifest over non-HTTPS URL: {0}")]
    InsecureUrl(String),
    #[error("Manifest signature missing or malformed: {0}")]
    BadSignatureFormat(&'static str),
    #[error(
        "Manifest signature verification failed — manifest does not match the embedded public key"
    )]
    BadSignature,
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

pub async fn fetch_manifest(http: &reqwest::Client, url: &str) -> Result<Manifest, ManifestError> {
    // Belt-and-braces with `Client::https_only(true)`: explicit prefix
    // check here gives a friendlier error than "https_only enforced by
    // policy" if a user types `http://...` in the manifest URL field.
    if !url.starts_with("https://") {
        return Err(ManifestError::InsecureUrl(url.to_string()));
    }
    let body = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    // Fetch the detached signature alongside the manifest. The signature
    // URL is conventionally `<manifest_url>.sig` and contains 128 hex
    // chars (64 raw bytes) — hex-encoded so the file is grep-able in
    // logs and CI output. If either fetch or verify fails we refuse the
    // manifest entirely; there is no unsigned fallback.
    let sig_url = format!("{url}.sig");
    let sig_text = http
        .get(&sig_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    verify_manifest_signature(&body, &sig_text)?;

    let manifest: Manifest = serde_json::from_slice(&body)?;
    manifest.validate()?;
    Ok(manifest)
}

/// Verify a detached hex-encoded Ed25519 signature against the manifest
/// body, using the embedded [`MANIFEST_SIGNING_PUBKEY`].
pub fn verify_manifest_signature(body: &[u8], sig_hex: &str) -> Result<(), ManifestError> {
    let sig_bytes = hex_decode_64(sig_hex).map_err(ManifestError::BadSignatureFormat)?;
    let signature = Signature::from_bytes(&sig_bytes);
    MANIFEST_SIGNING_PUBKEY
        .verify(body, &signature)
        .map_err(|_| ManifestError::BadSignature)
}

/// Resolves a blob path (e.g. `seed/sgw.zip`) against the manifest URL's
/// container.
///
/// Two modes:
///
/// 1. **Absolute** — if `blob_path` is itself a full URL (`http(s)://`)
///    it's returned unchanged. Lets the operator point individual
///    blobs at different hosts/releases — the GH Releases hosting
///    model puts the rolling `manifest.json` in a stable release tag
///    (`content-current`) while the immutable seed/patch blobs live
///    in per-publication release tags (`content-NNNN-…`), referenced
///    by absolute URL from the manifest.
///
/// 2. **Relative** — strip everything after the last `/` in the
///    manifest URL (and drop any `?query`/`#fragment` first, so a
///    SAS-tokened manifest URL still produces a clean base) and join
///    `blob_path` against the result. Backwards-compatible with the
///    previous Azure-Blob "everything in one container" layout.
pub fn blob_url(manifest_url: &str, blob_path: &str) -> String {
    if blob_path.starts_with("http://") || blob_path.starts_with("https://") {
        return blob_path.to_string();
    }
    let path_only = manifest_url
        .split_once('#')
        .map(|(left, _)| left)
        .unwrap_or(manifest_url);
    let path_only = path_only
        .split_once('?')
        .map(|(left, _)| left)
        .unwrap_or(path_only);
    let base = match path_only.rsplit_once('/') {
        Some((b, _)) => b,
        None => path_only,
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
    fn blob_url_strips_sas_query_before_resolving() {
        let u = blob_url(
            "https://x.blob.core.windows.net/sgw/manifest.json?sv=2024-11-01&sig=abc",
            "seed/s.zip",
        );
        assert_eq!(u, "https://x.blob.core.windows.net/sgw/seed/s.zip");
    }

    #[test]
    fn blob_url_strips_fragment_before_resolving() {
        let u = blob_url(
            "https://x.blob.core.windows.net/sgw/manifest.json#anchor",
            "seed/s.zip",
        );
        assert_eq!(u, "https://x.blob.core.windows.net/sgw/seed/s.zip");
    }

    #[test]
    fn blob_url_strips_both_query_and_fragment() {
        let u = blob_url(
            "https://x.blob.core.windows.net/sgw/manifest.json?sv=2024-11-01#x",
            "seed/s.zip",
        );
        assert_eq!(u, "https://x.blob.core.windows.net/sgw/seed/s.zip");
    }

    #[test]
    fn blob_url_passes_absolute_https_through() {
        // GH Releases mode: manifest in `content-current`, blob in an
        // immutable per-publication release tag.
        let u = blob_url(
            "https://github.com/Org/Repo/releases/download/content-current/manifest.json",
            "https://github.com/Org/Repo/releases/download/content-2026-05-20/seed.zip",
        );
        assert_eq!(
            u,
            "https://github.com/Org/Repo/releases/download/content-2026-05-20/seed.zip"
        );
    }

    #[test]
    fn blob_url_passes_absolute_http_through() {
        // Should not happen in production (https_only client refuses
        // http://) but the helper is responsible for URL resolution
        // only — the HTTPS check happens at fetch time.
        let u = blob_url(
            "https://x.com/manifest.json",
            "http://localhost:9000/seed.zip",
        );
        assert_eq!(u, "http://localhost:9000/seed.zip");
    }

    #[tokio::test]
    async fn fetch_manifest_rejects_non_https() {
        let http = reqwest::Client::new();
        let err = fetch_manifest(&http, "http://example.com/manifest.json")
            .await
            .unwrap_err();
        assert!(matches!(err, ManifestError::InsecureUrl(_)));
    }

    // ed25519_dalek's Signer trait is needed for SigningKey::sign().
    use ed25519_dalek::Signer;

    fn sign_with_dev_key(body: &[u8]) -> String {
        let sk = SigningKey::from_bytes(&DEV_MANIFEST_PRIVKEY);
        let sig = sk.sign(body);
        let bytes = sig.to_bytes();
        let mut s = String::with_capacity(128);
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    #[test]
    fn verify_manifest_signature_accepts_signed_body() {
        let body = br#"{"schema":1,"seed":{"blob":"s","size":1,"sha256":"h"}}"#;
        let sig_hex = sign_with_dev_key(body);
        verify_manifest_signature(body, &sig_hex).unwrap();
    }

    #[test]
    fn verify_manifest_signature_rejects_wrong_body() {
        let signed_body = br#"{"schema":1,"seed":{"blob":"a","size":1,"sha256":"h"}}"#;
        let sig_hex = sign_with_dev_key(signed_body);
        // Same signature, different bytes — must fail.
        let tampered = br#"{"schema":1,"seed":{"blob":"EVIL","size":1,"sha256":"h"}}"#;
        let err = verify_manifest_signature(tampered, &sig_hex).unwrap_err();
        assert!(matches!(err, ManifestError::BadSignature));
    }

    #[test]
    fn verify_manifest_signature_rejects_wrong_key() {
        let body = br#"{"schema":1,"seed":{"blob":"s","size":1,"sha256":"h"}}"#;
        // Sign with an unrelated key.
        let other = SigningKey::from_bytes(&[0x77; 32]);
        let sig = other.sign(body);
        let mut hex = String::with_capacity(128);
        for b in sig.to_bytes() {
            hex.push_str(&format!("{b:02x}"));
        }
        let err = verify_manifest_signature(body, &hex).unwrap_err();
        assert!(matches!(err, ManifestError::BadSignature));
    }

    #[test]
    fn verify_manifest_signature_rejects_malformed_hex() {
        let body = b"x";
        for bad in &["", "abc", "zzzz", &"a".repeat(127), &"a".repeat(129)] {
            let err = verify_manifest_signature(body, bad).unwrap_err();
            assert!(
                matches!(err, ManifestError::BadSignatureFormat(_)),
                "expected BadSignatureFormat for {bad:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn hex_decode_32_round_trip() {
        let bytes: [u8; 32] = [0xab; 32];
        let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let decoded = hex_decode_32(&hex).unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn parses_minimal_manifest_json() {
        let text = r#"{"schema":1,"seed":{"blob":"s","size":1,"sha256":"h"},"patches":[]}"#;
        let m: Manifest = serde_json::from_str(text).unwrap();
        m.validate().unwrap();
        assert_eq!(m.seed.blob, "s");
    }
}
