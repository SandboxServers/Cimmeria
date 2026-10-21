//! Publication checks for a candidate manifest and its file index (LT-35).
//!
//! Run before anyone signs: the manifest must parse and validate, its
//! `file_index` entry must describe the index bytes (same blob form as the
//! seed, the cap, size and digest, a verified parse, a binding to the
//! manifest), the chain from the current manifest must be append-only, and
//! the index must equal the one built from the local artifacts. Each check
//! yields one [`Finding`]; nothing here reads a key, signs or uploads.

use super::{sha256_hex, FileIndex, FileIndexRef, MAX_FILE_INDEX_BYTES};
use crate::manifest::Manifest;
use crate::manifest_chain::append_only;

/// The outcome of one publication check. `detail` is empty when `ok`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub check: &'static str,
    pub ok: bool,
    pub detail: String,
}

/// Checks, in order: manifest_parses, manifest_valid (Manifest::validate),
/// file_index_present, blob_form_matches_seed (same form as the seed's `blob`:
/// both absolute https URLs on one host, or both relative), index_under_cap,
/// index_size, index_digest, index_parse_verified, index_bind, then
/// append_only when `previous` is given, then parity when `built` is given
/// (the candidate index equals the one `file_index::build` makes from the
/// local seed and patches, compared as parsed values).
pub fn check_publication(
    candidate: &[u8],
    index: &[u8],
    previous: Option<&[u8]>,
    built: Option<&FileIndex>,
) -> Vec<Finding> {
    let mut out = Vec::new();
    let manifest: Manifest = match serde_json::from_slice(candidate) {
        Ok(manifest) => {
            out.push(finding("manifest_parses", Ok(())));
            manifest
        }
        Err(e) => {
            out.push(finding("manifest_parses", Err(e.to_string())));
            return out;
        }
    };
    out.push(finding(
        "manifest_valid",
        manifest.validate().map_err(|e| e.to_string()),
    ));

    let mut parsed = None;
    match manifest.file_index.as_ref() {
        None => out.push(finding(
            "file_index_present",
            Err("the manifest has no file_index entry".into()),
        )),
        Some(reference) => {
            out.push(finding("file_index_present", Ok(())));
            out.push(finding(
                "blob_form_matches_seed",
                blob_forms_match(&manifest.seed.blob, &reference.blob),
            ));
            out.push(finding("index_under_cap", under_cap(index)));
            out.push(finding("index_size", size_matches(index, reference)));
            out.push(finding("index_digest", digest_matches(index, reference)));
            let verified = FileIndex::parse_verified(index, reference);
            out.push(finding(
                "index_parse_verified",
                verified.as_ref().map(|_| ()).map_err(|e| e.to_string()),
            ));
            if let Ok(idx) = verified {
                out.push(finding(
                    "index_bind",
                    idx.bind(&manifest).map_err(|e| e.to_string()),
                ));
                parsed = Some(idx);
            }
        }
    }

    if let Some(previous) = previous {
        let result = match serde_json::from_slice::<Manifest>(previous) {
            Ok(prev) => append_only(&prev, &manifest).map_err(|b| format!("{b:?}")),
            Err(e) => Err(format!("the previous manifest does not parse: {e}")),
        };
        out.push(finding("append_only", result));
    }

    if let Some(built) = built {
        let result = match &parsed {
            Some(idx) if idx == built => Ok(()),
            Some(_) => Err("the index differs from the one built from the artifacts".into()),
            None => Err("the index did not verify, so it cannot be compared".into()),
        };
        out.push(finding("parity", result));
    }
    out
}

fn finding(check: &'static str, result: Result<(), String>) -> Finding {
    match result {
        Ok(()) => Finding {
            check,
            ok: true,
            detail: String::new(),
        },
        Err(detail) => Finding {
            check,
            ok: false,
            detail,
        },
    }
}

/// The two blob names have the same form: both absolute https URLs on one
/// host, or both relative.
fn blob_forms_match(seed: &str, index: &str) -> Result<(), String> {
    match (blob_form(seed), blob_form(index)) {
        (BlobForm::Relative, BlobForm::Relative) => Ok(()),
        (BlobForm::Https(a), BlobForm::Https(b)) if a.eq_ignore_ascii_case(b) => Ok(()),
        _ => Err(
            "the file index blob is not in the seed's form (same https host, or both relative)"
                .into(),
        ),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum BlobForm<'a> {
    Relative,
    Https(&'a str),
    Other,
}

fn blob_form(blob: &str) -> BlobForm<'_> {
    if !blob.contains("://") {
        return BlobForm::Relative;
    }
    match blob.strip_prefix("https://") {
        Some(rest) => match rest.split(['/', '?', '#']).next() {
            Some(host) if !host.is_empty() => BlobForm::Https(host),
            _ => BlobForm::Other,
        },
        None => BlobForm::Other,
    }
}

fn under_cap(index: &[u8]) -> Result<(), String> {
    if index.len() as u64 > MAX_FILE_INDEX_BYTES {
        return Err(format!(
            "{} bytes, over the {} byte cap",
            index.len(),
            MAX_FILE_INDEX_BYTES
        ));
    }
    Ok(())
}

fn size_matches(index: &[u8], reference: &FileIndexRef) -> Result<(), String> {
    if index.len() as u64 != reference.size {
        return Err(format!(
            "{} bytes, the manifest says {}",
            index.len(),
            reference.size
        ));
    }
    Ok(())
}

fn digest_matches(index: &[u8], reference: &FileIndexRef) -> Result<(), String> {
    if !sha256_hex(index).eq_ignore_ascii_case(&reference.sha256) {
        return Err("sha256 does not match the manifest's file_index entry".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_index::{
        IndexedFile, PatchLayer, SeedLayer, SetupRule, FILE_INDEX_SCHEMA, SETUP_POLICY_VERSION,
    };
    use crate::manifest::{PatchEntry, PatchRoot, SeedEntry};

    fn sha(c: char) -> String {
        c.to_string().repeat(64)
    }

    /// Seed SGW.exe; patch 001 overwrites it; patch 002 adds `Working/New.bin`.
    fn index() -> FileIndex {
        let file = |path: &str, c: char| IndexedFile {
            path: path.into(),
            size: 1,
            sha256: sha(c),
        };
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

    fn patch(id: &str, after: Option<&str>, c: char) -> PatchEntry {
        PatchEntry {
            id: id.into(),
            blob: format!("{id}.zip"),
            size: 1,
            sha256: sha(c),
            after: after.map(str::to_string),
            root: PatchRoot::InstallDir,
            title: None,
            description: None,
        }
    }

    /// A manifest whose `file_index` entry describes `bytes`, with the given patches.
    fn manifest(bytes: &[u8], patches: Vec<PatchEntry>) -> Manifest {
        Manifest {
            schema: 1,
            min_launcher: None,
            seed: SeedEntry {
                blob: "seed.zip".into(),
                size: 1,
                sha256: sha('1'),
            },
            patches,
            file_index: Some(FileIndexRef {
                blob: "index/file-index.json".into(),
                size: bytes.len() as u64,
                sha256: sha256_hex(bytes),
            }),
        }
    }

    fn two_patches() -> Vec<PatchEntry> {
        vec![patch("001", None, '2'), patch("002", Some("001"), '3')]
    }

    /// Candidate manifest bytes and index bytes that agree with each other.
    fn fixture() -> (Vec<u8>, Vec<u8>) {
        let index_bytes = serde_json::to_vec(&index()).unwrap();
        let candidate = serde_json::to_vec(&manifest(&index_bytes, two_patches())).unwrap();
        (candidate, index_bytes)
    }

    fn failed(findings: &[Finding]) -> Vec<&'static str> {
        findings.iter().filter(|f| !f.ok).map(|f| f.check).collect()
    }

    #[test]
    fn a_consistent_candidate_passes_every_check() {
        let (candidate, index_bytes) = fixture();
        let previous = serde_json::to_vec(&manifest(&[], two_patches())).unwrap();
        let findings = check_publication(&candidate, &index_bytes, Some(&previous), Some(&index()));
        let names: Vec<&str> = findings.iter().map(|f| f.check).collect();
        assert_eq!(
            names,
            [
                "manifest_parses",
                "manifest_valid",
                "file_index_present",
                "blob_form_matches_seed",
                "index_under_cap",
                "index_size",
                "index_digest",
                "index_parse_verified",
                "index_bind",
                "append_only",
                "parity",
            ]
        );
        assert!(failed(&findings).is_empty(), "{findings:?}");
    }

    #[test]
    fn a_flipped_digest_byte_fails_index_digest() {
        let (candidate, index_bytes) = fixture();
        let mut manifest: Manifest = serde_json::from_slice(&candidate).unwrap();
        let reference = manifest.file_index.as_mut().unwrap();
        let flipped = if reference.sha256.starts_with('f') {
            "0"
        } else {
            "f"
        };
        reference.sha256.replace_range(0..1, flipped);
        let candidate = serde_json::to_vec(&manifest).unwrap();
        let findings = check_publication(&candidate, &index_bytes, None, None);
        assert_eq!(failed(&findings), ["index_digest", "index_parse_verified"]);
    }

    #[test]
    fn a_wrong_file_index_size_fails_index_size() {
        let (candidate, index_bytes) = fixture();
        let mut manifest: Manifest = serde_json::from_slice(&candidate).unwrap();
        manifest.file_index.as_mut().unwrap().size += 1;
        let candidate = serde_json::to_vec(&manifest).unwrap();
        let findings = check_publication(&candidate, &index_bytes, None, None);
        assert!(failed(&findings).contains(&"index_size"), "{findings:?}");
    }

    #[test]
    fn a_reordered_patch_with_previous_fails_append_only() {
        let (_, index_bytes) = fixture();
        let previous = serde_json::to_vec(&manifest(&index_bytes, two_patches())).unwrap();
        let reordered = vec![patch("002", None, '3'), patch("001", None, '2')];
        let candidate = serde_json::to_vec(&manifest(&index_bytes, reordered)).unwrap();
        let findings = check_publication(&candidate, &index_bytes, Some(&previous), None);
        // index_bind also refuses the reordered list; append_only is the check under test.
        assert!(failed(&findings).contains(&"append_only"), "{findings:?}");
    }

    #[test]
    fn an_index_from_other_artifacts_fails_parity() {
        let (candidate, index_bytes) = fixture();
        let mut other = index();
        other.patches[1].sha256 = sha('9');
        let findings = check_publication(&candidate, &index_bytes, None, Some(&other));
        assert_eq!(failed(&findings), ["parity"]);
    }

    #[test]
    fn a_relative_index_blob_under_an_https_seed_fails_the_blob_form() {
        let (candidate, index_bytes) = fixture();
        let mut manifest: Manifest = serde_json::from_slice(&candidate).unwrap();
        manifest.seed.blob = "https://play.example/seed.zip".into();
        let candidate = serde_json::to_vec(&manifest).unwrap();
        let findings = check_publication(&candidate, &index_bytes, None, None);
        assert_eq!(failed(&findings), ["blob_form_matches_seed"]);
    }
}
