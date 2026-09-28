//! Client patch sets for the SGW launcher.
//!
//! A patch set is an ordinary patch zip carrying a `cimmeria-patch.json`
//! [`Recipe`]. Each recipe op rebuilds one client file from files the
//! player already has, the stock client unpacked from the archive.org
//! seed, plus a bsdiff delta:
//!
//! ```text
//! result = bspatch(concat(transform(source_1), transform(source_2), …), delta)
//! ```
//!
//! so the zip holds only the bytes Cimmeria authored, never a copy of a
//! CME file. Every source and every result is checked by SHA-256, and an op
//! whose target already has the result hash is skipped, so applying a
//! patch set twice is a no-op. Zip entries other than the recipe and its
//! deltas are plain overlay files (content that is entirely ours).
//!
//! The one [`Transform`], [`Transform::UpkNormalize`], exists because our
//! patched maps are written uncompressed by `cimmeria-upk`'s append-only
//! patcher while the stock maps are LZO-compressed: diffing against the
//! stock bytes would drag the whole decompressed map into the delta.
//! Normalizing the stock map first (open + write back, no changes) gives
//! the patcher's own starting image, and the delta shrinks to what the
//! patch appended.
//!
//! [`build`] makes a patch zip from a spec, [`apply`] applies one, and
//! [`signing`] signs and verifies launcher manifests. The
//! `cimmeria-patchset` binary wraps all three.

pub mod apply;
pub mod build;
pub mod recipe;
pub mod signing;
pub mod transform;

pub use apply::{apply, ApplyReport};
pub use build::{build, Spec};
pub use recipe::{Op, Recipe, Source, Transform, RECIPE_NAME};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PatchsetError {
    #[error("IO error on {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Could not normalize package {path}: {detail}")]
    Upk { path: String, detail: String },
    #[error("Delta error for {target}: {detail}")]
    Delta { target: String, detail: String },
    #[error(
        "{path} does not match the stock client (expected sha256 {expected}, found {actual}). \
         Reinstall the seed, or restore the original file."
    )]
    SourceMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("{path} is missing; this patch rebuilds it from the stock client")]
    SourceMissing { path: String },
    #[error("Rebuilt {path} has sha256 {actual}, expected {expected}")]
    ResultMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("Unsupported recipe schema {0} (this build understands 1)")]
    UnsupportedSchema(u32),
    #[error("Recipe path {0:?} would land outside the install directory")]
    UnsafePath(String),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, PatchsetError>;

pub(crate) fn io_err(path: &std::path::Path) -> impl FnOnce(std::io::Error) -> PatchsetError + '_ {
    move |source| PatchsetError::Io {
        path: path.display().to_string(),
        source,
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests;
