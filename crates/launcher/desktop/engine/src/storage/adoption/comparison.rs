use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn compare(
    source_root: &Path,
    source: &inventory::Index,
    reference: &reference::Reference,
) -> Result<Vec<Difference>, Error> {
    let binaries = crate::install_layout::binaries_dir(source_root);
    let full = source_root.join("Working").is_dir();
    let working = std::fs::read_dir(source_root)?.any(|entry| {
        entry.is_ok_and(|entry| {
            entry.file_name().eq_ignore_ascii_case("binaries")
                && entry.file_type().is_ok_and(|kind| kind.is_dir())
        })
    });
    let flat = source.keys().any(|p| p.eq_ignore_ascii_case("SGW.exe"))
        || (!full && !working && source_root.join("SGWGame").is_dir());
    if [full, working, flat].into_iter().filter(|v| *v).count() != 1
        || !binaries.starts_with(source_root)
    {
        return Err(StorageError::InvalidDirectory.into());
    }
    let source_names: BTreeMap<_, _> = source.keys().map(|p| (p.to_ascii_lowercase(), p)).collect();
    let raw_names: BTreeMap<_, _> = reference
        .raw
        .iter()
        .map(|(p, e)| (p.to_ascii_lowercase(), e))
        .collect();
    let mut used = BTreeSet::new();
    let mut differences = Vec::new();
    for (path, expected) in &reference.index {
        // The synthesized copy ledger is never reused from the historical source.
        if path == "launcher-installed.json" {
            continue;
        }
        let mapped = if full {
            path.as_str()
        } else if let Some(relative) = path.strip_prefix("Working/") {
            if flat {
                relative.strip_prefix("Binaries/").unwrap_or(relative)
            } else {
                relative
            }
        } else {
            path.as_str()
        };
        let found = source_names.get(&mapped.to_ascii_lowercase()).copied();
        let classification = match found {
            None => Classification::Missing,
            Some(source_path) => {
                used.insert(source_path.clone());
                let entry = &source[source_path];
                if entry.sha256 == expected.sha256 && entry.size == expected.size {
                    if source_path == path {
                        Classification::Matched
                    } else {
                        Classification::KnownTransform
                    }
                } else if raw_names
                    .get(&path.to_ascii_lowercase())
                    .is_some_and(|raw| raw.sha256 == entry.sha256 && raw.size == entry.size)
                {
                    Classification::KnownTransform
                } else {
                    Classification::Modified
                }
            }
        };
        differences.push(Difference {
            path: path.clone(),
            source_path: found.cloned(),
            classification,
        });
    }
    for path in source.keys().filter(|p| !used.contains(*p)) {
        differences.push(Difference {
            path: path.clone(),
            source_path: Some(path.clone()),
            classification: Classification::Extra,
        });
    }
    Ok(differences)
}
