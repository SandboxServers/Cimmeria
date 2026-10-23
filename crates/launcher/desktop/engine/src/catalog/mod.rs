//! Authenticated release patch notes. This is catalog data, not installed state.
use crate::manifest::{self, Manifest, ManifestError};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

mod events;

pub(crate) const URL: &str =
    "https://github.com/SandboxServers/Cimmeria/releases/download/content-current/manifest.json";
const MAX_BODY: usize = 1024 * 1024;
const MAX_SIGNATURE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogError {
    Network,
    TooLarge,
    Signature,
    SigningKeyUnavailable,
    /// A debug build's default fixture key cannot authenticate the public catalog.
    DevelopmentSigningKey,
    InvalidManifest,
}

#[derive(Debug, Serialize)]
pub struct PatchNotes {
    pub schema_version: u32,
    pub patches: Vec<PatchNote>,
}
#[derive(Debug, Serialize)]
pub struct PatchNote {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
}

/// Fixed native-owned origin; the webview cannot choose arbitrary fetch targets.
/// Each request has a finite total deadline, including body consumption.
pub async fn fetch_patch_notes() -> Result<PatchNotes, CatalogError> {
    Ok(notes(fetch_release().await?.manifest))
}

/// Authenticated native input; cannot be constructed by deserializing a UI request.
#[derive(Debug)]
pub struct VerifiedRelease {
    signed_body: Vec<u8>,
    signature: Vec<u8>,
    manifest: Manifest,
    digest: [u8; 32],
}
impl VerifiedRelease {
    pub(crate) fn evidence(&self) -> (&[u8], &[u8]) {
        (&self.signed_body, &self.signature)
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

pub async fn fetch_release() -> Result<VerifiedRelease, CatalogError> {
    require_release_key(
        manifest::MANIFEST_SIGNING_PUBKEY.is_some(),
        cfg!(debug_assertions)
            && option_env!("LAUNCHER_MANIFEST_PUBKEY_HEX").is_none_or(|key| key.trim().is_empty()),
    )?;
    let started = Instant::now();
    let client = reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| {
            events::release_fetch_failed("network", None);
            CatalogError::Network
        })?;
    let body = read_bounded(&client, URL, MAX_BODY).await?;
    let signature = read_bounded(&client, &manifest::sig_url_for(URL), MAX_SIGNATURE).await?;
    let release = verify_fetched(&body, &signature)?;
    events::release_fetched(started.elapsed());
    Ok(release)
}

/// The fixed public endpoint requires a release key. Local signed fixtures use
/// `verify_release` directly and retain their development-key verification.
fn require_release_key(present: bool, development_default: bool) -> Result<(), CatalogError> {
    let failure = if !present {
        Some((
            CatalogError::SigningKeyUnavailable,
            "signing_key_unavailable",
        ))
    } else if development_default {
        Some((CatalogError::DevelopmentSigningKey, "development_key"))
    } else {
        None
    };
    if let Some((error, reason)) = failure {
        events::release_fetch_failed(reason, None);
        return Err(error);
    }
    Ok(())
}

async fn read_bounded(
    client: &reqwest::Client,
    url: &str,
    limit: usize,
) -> Result<Vec<u8>, CatalogError> {
    let mut response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| {
            let http_status = error.status().map(|status| status.as_u16());
            let reason = if http_status.is_some() {
                "http_status"
            } else {
                "network"
            };
            events::release_fetch_failed(reason, http_status);
            CatalogError::Network
        })?;
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        events::release_fetch_failed("body", None);
        return Err(CatalogError::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        events::release_fetch_failed("network", None);
        CatalogError::Network
    })? {
        if chunk.len() > limit.saturating_sub(body.len()) {
            events::release_fetch_failed("body", None);
            return Err(CatalogError::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Verify original signed bytes before retaining their identity for installation.
pub fn verify_release(body: &[u8], signature: &[u8]) -> Result<VerifiedRelease, CatalogError> {
    Ok(VerifiedRelease {
        manifest: decode_verified(body, signature)?,
        signed_body: body.to_vec(),
        signature: signature.to_vec(),
        digest: Sha256::digest(body).into(),
    })
}

fn decode_verified(body: &[u8], signature: &[u8]) -> Result<Manifest, CatalogError> {
    if body.len() > MAX_BODY || signature.len() > MAX_SIGNATURE {
        return Err(CatalogError::TooLarge);
    }
    let signature = std::str::from_utf8(signature)
        .map_err(|_| CatalogError::Signature)?
        .trim();
    // Validate ASCII before the shared hex parser uses byte-indexed string slices.
    if signature.len() != 128 || !signature.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(CatalogError::Signature);
    }
    manifest::verify_manifest_signature(body, signature).map_err(|error| match error {
        ManifestError::SigningKeyUnavailable => CatalogError::SigningKeyUnavailable,
        _ => CatalogError::Signature,
    })?;
    let manifest: Manifest =
        serde_json::from_slice(body).map_err(|_| CatalogError::InvalidManifest)?;
    manifest
        .validate()
        .map_err(|_| CatalogError::InvalidManifest)?;
    Ok(manifest)
}

/// Verify a body fetched from the release server. Its failure is the fetch failure row.
fn verify_fetched(body: &[u8], signature: &[u8]) -> Result<VerifiedRelease, CatalogError> {
    verify_release(body, signature).inspect_err(|&error| {
        let reason = match error {
            CatalogError::InvalidManifest => "invalid_manifest",
            CatalogError::TooLarge => "body",
            _ => "signature",
        };
        events::release_fetch_failed(reason, None);
    })
}

fn notes(manifest: Manifest) -> PatchNotes {
    PatchNotes {
        schema_version: 1,
        patches: manifest
            .patches
            .into_iter()
            .map(|patch| PatchNote {
                title: patch
                    .title
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or_else(|| patch.id.clone()),
                id: patch.id,
                description: patch.description,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests;
