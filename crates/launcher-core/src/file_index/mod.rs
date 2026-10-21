//! The signed per-file index (any-install D-AI2).
//!
//! The index is the launcher's reference reconstruction written down: the
//! files the seed produces, then the files each signed patch reads and
//! writes, then the setup rules. Paths are relative to the client root (the
//! folder holding `Working`), `/`-separated, in the stock spelling
//! `client_setup::stock_case` restores. The manifest signs the index's
//! digest through [`FileIndexRef`], so the index bytes are trusted only
//! after [`FileIndex::parse_verified`] has checked them.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::manifest::Manifest;

mod build;
#[cfg(test)]
mod build_tests;

pub use build::build;

/// Publication checks for a candidate manifest and index (LT-35).
pub mod check;

/// The only index schema this launcher reads.
pub const FILE_INDEX_SCHEMA: u32 = 1;
/// Bumped when client_setup changes what it writes. A launcher ignores an
/// index whose policy it does not implement and builds one locally instead.
pub const SETUP_POLICY_VERSION: u32 = 1;
/// The blob's cap. The real index is about 1 MB (README F-9).
pub const MAX_FILE_INDEX_BYTES: u64 = 16 * 1024 * 1024;

/// In the signed manifest. Optional, so no schema bump (the `min_launcher` precedent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileIndexRef {
    pub blob: String,
    pub size: u64,
    pub sha256: String,
}

/// The whole index: one JSON document, see the module docs for its shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileIndex {
    pub schema_version: u32,
    pub setup_policy_version: u32,
    pub seed: SeedLayer,
    pub patches: Vec<PatchLayer>,
    pub setup: Vec<SetupRule>,
}

/// The files the seed produces after `place_bundled_cooked_data`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedLayer {
    pub sha256: String,
    pub files: Vec<IndexedFile>,
}

/// One signed patch: the files it reads at their pre-patch bytes and the
/// files it writes at their post-patch bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchLayer {
    pub state_key: String,
    pub sha256: String,
    #[serde(default)]
    pub inputs: Vec<IndexedFile>,
    pub outputs: Vec<IndexedFile>,
}

/// A file's path, size and SHA-256 at one layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

/// A setup rule: a client file that client_setup rewrites after the patches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case", deny_unknown_fields)]
pub enum SetupRule {
    AslrOff {
        path: String,
        size: u64,
        sha256: String,
    },
    LoginServers {
        path: String,
    },
}

/// Why an index was refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FileIndexError {
    #[error("file index is larger than its cap")]
    TooLarge,
    #[error("file index bytes do not match the signed reference")]
    Digest,
    #[error("unsupported file index schema {0}")]
    Schema(u32),
    #[error("unsupported setup policy {0}")]
    SetupPolicy(u32),
    #[error("file index does not describe this release: {0}")]
    Binding(String),
    #[error("invalid path in file index: {0}")]
    InvalidPath(String),
    #[error("duplicate path in one layer: {0}")]
    Duplicate(String),
    #[error("{0}")]
    Io(String),
}

/// Where one path's bytes come from at one layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Seed,
    /// The index of the patch in the manifest's `patches` list.
    Patch(usize),
}

/// One layer's bytes for a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub origin: Origin,
    pub size: u64,
    pub sha256: [u8; 32],
}

/// Every state a path passes through, oldest first. `release()` is the last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathHistory {
    pub path: String,
    pub states: Vec<State>,
}

impl FileIndex {
    /// Size and SHA-256 against the signed reference first, then parse, then `validate`.
    pub fn parse_verified(bytes: &[u8], reference: &FileIndexRef) -> Result<Self, FileIndexError> {
        if bytes.len() as u64 > MAX_FILE_INDEX_BYTES {
            return Err(FileIndexError::TooLarge);
        }
        if bytes.len() as u64 != reference.size
            || !sha256_hex(bytes).eq_ignore_ascii_case(&reference.sha256)
        {
            return Err(FileIndexError::Digest);
        }
        let index: FileIndex = serde_json::from_slice(bytes)
            .map_err(|e| FileIndexError::Io(format!("file index is not valid JSON: {e}")))?;
        index.validate()?;
        Ok(index)
    }

    /// Schema, setup policy, path rules (relative, no `..`, `.`, empty, `\`, `:`
    /// or leading `/`, ASCII), no duplicate path within one layer (case-blind).
    /// Hashes must be 64 lower-case hex characters.
    pub fn validate(&self) -> Result<(), FileIndexError> {
        if self.schema_version != FILE_INDEX_SCHEMA {
            return Err(FileIndexError::Schema(self.schema_version));
        }
        if self.setup_policy_version != SETUP_POLICY_VERSION {
            return Err(FileIndexError::SetupPolicy(self.setup_policy_version));
        }
        check_hex(&self.seed.sha256)?;
        check_layer(&self.seed.files)?;
        for patch in &self.patches {
            check_hex(&patch.sha256)?;
            check_layer(&patch.inputs)?;
            check_layer(&patch.outputs)?;
        }
        for rule in &self.setup {
            match rule {
                SetupRule::AslrOff { path, sha256, .. } => {
                    check_path(path)?;
                    check_hex(sha256)?;
                }
                SetupRule::LoginServers { path } => check_path(path)?,
            }
        }
        Ok(())
    }

    /// Seed sha256 equal (case-blind) and the patch list equal to the manifest's,
    /// position by position: `state_key()` and `sha256`.
    pub fn bind(&self, manifest: &Manifest) -> Result<(), FileIndexError> {
        if !self.seed.sha256.eq_ignore_ascii_case(&manifest.seed.sha256) {
            return Err(FileIndexError::Binding(
                "seed sha256 differs from the manifest".into(),
            ));
        }
        if self.patches.len() != manifest.patches.len() {
            return Err(FileIndexError::Binding(format!(
                "index lists {} patches, the manifest {}",
                self.patches.len(),
                manifest.patches.len()
            )));
        }
        for (layer, entry) in self.patches.iter().zip(&manifest.patches) {
            if layer.state_key != entry.state_key() {
                return Err(FileIndexError::Binding(format!(
                    "index patch '{}' where the manifest has '{}'",
                    layer.state_key,
                    entry.state_key()
                )));
            }
            if !layer.sha256.eq_ignore_ascii_case(&entry.sha256) {
                return Err(FileIndexError::Binding(format!(
                    "index sha256 for patch '{}' differs from the manifest",
                    layer.state_key
                )));
            }
        }
        Ok(())
    }

    /// Keyed by lower-cased path. Seed files give the first state; each patch
    /// output gives the next one, in patch order.
    pub fn histories(&self) -> BTreeMap<String, PathHistory> {
        let mut out = BTreeMap::new();
        for file in &self.seed.files {
            push_state(&mut out, file, Origin::Seed);
        }
        for (index, patch) in self.patches.iter().enumerate() {
            for file in &patch.outputs {
                push_state(&mut out, file, Origin::Patch(index));
            }
        }
        out
    }
}

fn push_state(out: &mut BTreeMap<String, PathHistory>, file: &IndexedFile, origin: Origin) {
    let history = out
        .entry(file.path.to_ascii_lowercase())
        .or_insert_with(|| PathHistory {
            path: file.path.clone(),
            states: Vec::new(),
        });
    history.states.push(State {
        origin,
        size: file.size,
        sha256: decode_sha256(&file.sha256),
    });
}

/// A path is relative and plain ASCII with no `\`, `:`, leading `/`, empty
/// or `.`/`..` component.
fn check_path(path: &str) -> Result<(), FileIndexError> {
    let bad = path.is_empty()
        || !path.is_ascii()
        || path.starts_with('/')
        || path.contains(['\\', ':'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..");
    if bad {
        return Err(FileIndexError::InvalidPath(path.to_string()));
    }
    Ok(())
}

fn check_hex(sha: &str) -> Result<(), FileIndexError> {
    let ok = sha.len() == 64
        && sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !ok {
        return Err(FileIndexError::Binding(format!(
            "sha256 is not 64 lower-case hex characters: {sha}"
        )));
    }
    Ok(())
}

fn check_layer(files: &[IndexedFile]) -> Result<(), FileIndexError> {
    let mut seen = HashSet::new();
    for file in files {
        check_path(&file.path)?;
        check_hex(&file.sha256)?;
        if !seen.insert(file.path.to_ascii_lowercase()) {
            return Err(FileIndexError::Duplicate(file.path.clone()));
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Decodes a validated 64-char hex hash. A malformed one reads as zeros;
/// `validate` refuses those before any history is built.
fn decode_sha256(hex: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = hex
            .get(i * 2..i * 2 + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .unwrap_or(0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(c: char) -> String {
        c.to_string().repeat(64)
    }

    fn file(path: &str, c: char) -> IndexedFile {
        IndexedFile {
            path: path.into(),
            size: 1,
            sha256: sha(c),
        }
    }

    /// Seed SGW.exe; patch 0 overwrites it; patch 1 adds `Working/New.bin`.
    fn index() -> FileIndex {
        FileIndex {
            schema_version: FILE_INDEX_SCHEMA,
            setup_policy_version: SETUP_POLICY_VERSION,
            seed: SeedLayer {
                sha256: sha('1'),
                files: vec![file("Working/Binaries/SGW.exe", 'a')],
            },
            patches: vec![
                PatchLayer {
                    state_key: "001".into(),
                    sha256: sha('2'),
                    inputs: vec![],
                    outputs: vec![file("Working/Binaries/SGW.exe", 'b')],
                },
                PatchLayer {
                    state_key: "002".into(),
                    sha256: sha('3'),
                    inputs: vec![],
                    outputs: vec![file("Working/New.bin", 'c')],
                },
            ],
            setup: vec![SetupRule::LoginServers {
                path: "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua".into(),
            }],
        }
    }

    /// A manifest whose seed is `seed` and whose patches are `(id, sha)` pairs.
    fn manifest(seed: char, patches: &[(&str, char)]) -> Manifest {
        let entries: Vec<String> = patches
            .iter()
            .map(|(id, c)| {
                format!(
                    r#"{{"id":"{id}","blob":"p.zip","size":1,"sha256":"{}"}}"#,
                    sha(*c)
                )
            })
            .collect();
        let json = format!(
            r#"{{"schema":1,"seed":{{"blob":"s.zip","size":1,"sha256":"{}"}},"patches":[{}]}}"#,
            sha(seed),
            entries.join(",")
        );
        serde_json::from_str(&json).unwrap()
    }

    fn with_seed_path(path: &str) -> FileIndex {
        let mut idx = index();
        idx.seed.files[0].path = path.into();
        idx
    }

    #[test]
    fn parse_verified_accepts_the_signed_bytes() {
        let json = serde_json::to_vec(&index()).unwrap();
        let reference = FileIndexRef {
            blob: "index/file-index.json".into(),
            size: json.len() as u64,
            sha256: sha256_hex(&json),
        };
        assert_eq!(
            FileIndex::parse_verified(&json, &reference).unwrap(),
            index()
        );
    }

    #[test]
    fn parse_verified_refuses_one_flipped_byte() {
        let mut json = serde_json::to_vec(&index()).unwrap();
        let reference = FileIndexRef {
            blob: "index/file-index.json".into(),
            size: json.len() as u64,
            sha256: sha256_hex(&json),
        };
        let mid = json.len() / 2;
        json[mid] ^= 0x01;
        assert!(matches!(
            FileIndex::parse_verified(&json, &reference),
            Err(FileIndexError::Digest)
        ));
    }

    #[test]
    fn parse_verified_refuses_a_body_over_the_cap() {
        let body = vec![b' '; (MAX_FILE_INDEX_BYTES + 1) as usize];
        let reference = FileIndexRef {
            blob: "index/file-index.json".into(),
            size: body.len() as u64,
            sha256: sha('a'),
        };
        assert!(matches!(
            FileIndex::parse_verified(&body, &reference),
            Err(FileIndexError::TooLarge)
        ));
    }

    #[test]
    fn bind_accepts_a_matching_manifest() {
        assert!(index()
            .bind(&manifest('1', &[("001", '2'), ("002", '3')]))
            .is_ok());
    }

    #[test]
    fn bind_refuses_a_different_seed_sha() {
        let m = manifest('4', &[("001", '2'), ("002", '3')]);
        assert!(matches!(index().bind(&m), Err(FileIndexError::Binding(_))));
    }

    #[test]
    fn bind_refuses_a_missing_patch() {
        let m = manifest('1', &[("001", '2')]);
        assert!(matches!(index().bind(&m), Err(FileIndexError::Binding(_))));
    }

    #[test]
    fn bind_refuses_a_reordered_patch_list() {
        let m = manifest('1', &[("002", '3'), ("001", '2')]);
        assert!(matches!(index().bind(&m), Err(FileIndexError::Binding(_))));
    }

    #[test]
    fn bind_refuses_a_patch_with_another_sha() {
        let m = manifest('1', &[("001", '5'), ("002", '3')]);
        assert!(matches!(index().bind(&m), Err(FileIndexError::Binding(_))));
    }

    #[test]
    fn validate_refuses_a_parent_component() {
        assert!(matches!(
            with_seed_path("../x").validate(),
            Err(FileIndexError::InvalidPath(_))
        ));
    }

    #[test]
    fn validate_refuses_a_backslash() {
        assert!(matches!(
            with_seed_path("a\\b").validate(),
            Err(FileIndexError::InvalidPath(_))
        ));
    }

    #[test]
    fn validate_refuses_a_drive_letter() {
        assert!(matches!(
            with_seed_path("C:/x").validate(),
            Err(FileIndexError::InvalidPath(_))
        ));
    }

    #[test]
    fn validate_refuses_paths_differing_only_by_case_in_one_layer() {
        let mut idx = index();
        idx.seed.files.push(file("working/binaries/sgw.EXE", 'd'));
        assert!(matches!(idx.validate(), Err(FileIndexError::Duplicate(_))));
    }

    #[test]
    fn validate_accepts_the_sample_index() {
        assert!(index().validate().is_ok());
    }

    #[test]
    fn histories_chain_a_seed_file_through_a_patch_overwrite() {
        let h = index().histories();
        let sgw = &h["working/binaries/sgw.exe"];
        assert_eq!(sgw.path, "Working/Binaries/SGW.exe");
        assert_eq!(
            sgw.states,
            vec![
                State {
                    origin: Origin::Seed,
                    size: 1,
                    sha256: [0xaa; 32],
                },
                State {
                    origin: Origin::Patch(0),
                    size: 1,
                    sha256: [0xbb; 32],
                },
            ]
        );
    }

    #[test]
    fn histories_give_a_patch_added_file_one_state() {
        let h = index().histories();
        let added = &h["working/new.bin"];
        assert_eq!(
            added.states,
            vec![State {
                origin: Origin::Patch(1),
                size: 1,
                sha256: [0xcc; 32],
            }]
        );
    }
}
