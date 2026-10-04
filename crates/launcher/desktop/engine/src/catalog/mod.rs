//! Authenticated release patch notes. This is catalog data, not installed state.
use crate::manifest::{self, Manifest, ManifestError};
use serde::Serialize;
use std::time::Duration;

const URL: &str =
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
    if manifest::MANIFEST_SIGNING_PUBKEY.is_none() {
        return Err(CatalogError::SigningKeyUnavailable);
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| CatalogError::Network)?;
    let body = read_bounded(&client, URL, MAX_BODY).await?;
    let signature = read_bounded(&client, &manifest::sig_url_for(URL), MAX_SIGNATURE).await?;
    let manifest = decode_verified(&body, &signature)?;
    Ok(notes(manifest))
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
        .map_err(|_| CatalogError::Network)?;
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(CatalogError::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| CatalogError::Network)? {
        if chunk.len() > limit.saturating_sub(body.len()) {
            return Err(CatalogError::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
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
