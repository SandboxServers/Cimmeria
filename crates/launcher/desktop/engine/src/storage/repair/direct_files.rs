//! Journaled per-file promotion for adopted roots. Never rename the install tree.
use super::*;
use crate::storage::adoption::direct_paths::Parents;
use std::io::{Read, Write};

fn storage(error: adoption::Error) -> StorageError {
    match error {
        adoption::Error::Storage(error) => error,
        _ => StorageError::Corrupt,
    }
}
fn retained_hash(parents: &Parents, path: &Path) -> Result<Option<[u8; 32]>, StorageError> {
    if !parents.present(path).map_err(storage)? {
        return Ok(None);
    }
    Ok(Some(parents.hash(path).map_err(storage)?.sha256))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BackupBoundary {
    Created,
    PrefixCopied,
    Published,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: PathBuf,
    original: Option<[u8; 32]>,
    replacement: Option<[u8; 32]>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal<P> {
    schema_version: u32,
    plan: P,
    entries: Vec<Entry>,
}

pub(crate) fn apply<P: tree_identity::TreePlan>(
    plan: &P,
    root: &Path,
    stage: &Path,
    backup: &Path,
    work: &Path,
    previous: &[PathBuf],
    owner: &File,
) -> Result<(), StorageError> {
    apply_with_owner(plan, root, stage, backup, work, previous, owner, |_| Ok(()))
}

pub(crate) fn verify<P: tree_identity::TreePlan>(
    plan: &P,
    root: &Path,
    work: &Path,
) -> Result<(), StorageError> {
    let journal: Journal<P> =
        read(&work.join("direct-files.json"))?.ok_or(StorageError::Corrupt)?;
    if !matches!(journal.schema_version, 1 | 2) || journal.plan != *plan {
        return Err(StorageError::Corrupt);
    }
    for entry in journal.entries {
        validate_relative(&entry.path)?;
        checked(root, &entry.path)?;
        if hash(&root.join(entry.path))? != entry.replacement {
            return Err(StorageError::Corrupt);
        }
    }
    Ok(())
}

pub(crate) fn paths<P: tree_identity::TreePlan>(
    plan: &P,
    work: &Path,
) -> Result<Vec<PathBuf>, StorageError> {
    let journal: Journal<P> =
        read(&work.join("direct-files.json"))?.ok_or(StorageError::Corrupt)?;
    if !matches!(journal.schema_version, 1 | 2) || journal.plan != *plan {
        return Err(StorageError::Corrupt);
    }
    Ok(journal
        .entries
        .into_iter()
        .filter(|entry| entry.replacement.is_some() && entry.original != entry.replacement)
        .map(|entry| entry.path)
        .collect())
}

fn apply_with_owner<P: tree_identity::TreePlan>(
    plan: &P,
    root: &Path,
    stage: &Path,
    backup: &Path,
    work: &Path,
    previous: &[PathBuf],
    owner: &File,
    mut after_backup: impl FnMut(BackupBoundary) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    let journal_path = work.join("direct-files.json");
    let enumerate = stage.join(".enumerate");
    let preparation =
        Parents::for_owner(root, &[&journal_path, &enumerate], false, owner).map_err(storage)?;
    let saved: Option<Journal<P>> = if preparation.present(&journal_path).map_err(storage)? {
        Some(
            serde_json::from_slice(&preparation.read(&journal_path).map_err(storage)?)
                .map_err(|_| StorageError::Corrupt)?,
        )
    } else {
        None
    };
    let journal = match saved {
        Some(saved) if matches!(saved.schema_version, 1 | 2) && saved.plan == *plan => saved,
        Some(_) => return Err(StorageError::Corrupt),
        None => {
            if directory_or_absent(backup)? {
                return Err(StorageError::UnsafeFile);
            }
            let mut paths = Vec::new();
            collect(stage, stage, &mut paths)?;
            let targets: std::collections::BTreeSet<_> = paths.iter().cloned().collect();
            paths.extend(
                previous
                    .iter()
                    .filter(|path| !targets.contains(*path))
                    .cloned(),
            );
            paths.sort();
            paths.dedup();
            let mut entries = Vec::new();
            for path in paths {
                validate_relative(&path)?;
                checked(root, &path)?;
                entries.push(Entry {
                    original: hash(&root.join(&path))?,
                    replacement: hash(&stage.join(&path))?,
                    path,
                });
            }
            let saved = Journal {
                schema_version: 2,
                plan: plan.clone(),
                entries,
            };
            preparation
                .write(
                    &journal_path,
                    &serde_json::to_vec(&saved).map_err(|_| StorageError::Corrupt)?,
                )
                .map_err(storage)?;
            commit::sync(work)?;
            saved
        }
    };
    let mut mutation_paths = vec![
        backup.join(tree_identity::name(plan)),
        work.join("direct-files.json"),
    ];
    for entry in &journal.entries {
        validate_relative(&entry.path)?;
        mutation_paths.extend([
            root.join(&entry.path),
            stage.join(&entry.path),
            backup.join(&entry.path),
        ]);
    }
    let marker_path = backup.join(tree_identity::name(plan));
    let bootstrap = Parents::for_owner(root, &[&marker_path], true, owner).map_err(storage)?;
    if !bootstrap.present(&marker_path).map_err(storage)? {
        if !bootstrap.names(backup).map_err(storage)?.is_empty() {
            return Err(StorageError::Corrupt);
        }
        let marker = serde_json::json!({"schema_version":1,"plan":plan,"role":"original"});
        let mut output = bootstrap.open(&marker_path, true, true).map_err(storage)?;
        output
            .write_all(&serde_json::to_vec(&marker).map_err(|_| StorageError::Corrupt)?)
            .map_err(|_| StorageError::Io)?;
        output
            .sync_all()
            .map_err(|_| StorageError::PersistenceUncertain)?;
        commit::sync(backup)?;
    } else {
        let marker: serde_json::Value =
            serde_json::from_slice(&bootstrap.read(&marker_path).map_err(storage)?)
                .map_err(|_| StorageError::Corrupt)?;
        if marker != serde_json::json!({"schema_version":1,"plan":plan,"role":"original"}) {
            return Err(StorageError::Corrupt);
        }
    }
    drop(bootstrap);
    let borrowed: Vec<_> = mutation_paths.iter().map(PathBuf::as_path).collect();
    let parents = Parents::for_owner(root, &borrowed, true, owner).map_err(storage)?;
    let held_paths: std::collections::BTreeSet<_> = journal
        .entries
        .iter()
        .map(|entry| backup.join(&entry.path))
        .collect();
    for entry in &journal.entries {
        let partial = removal_partial(&backup.join(&entry.path));
        if held_paths.contains(&partial) {
            return Err(StorageError::Corrupt);
        }
    }
    // The immutable journal is complete before any original file is touched.
    for entry in &journal.entries {
        validate_relative(&entry.path)?;
        let live = root.join(&entry.path);
        let staged = stage.join(&entry.path);
        let held = backup.join(&entry.path);
        if entry.original.is_some() && entry.original == entry.replacement {
            // Windows incremental stages can hard-link unchanged live files.
            // Discard that journal-owned alias before enforcing the live file's
            // single-link invariant; never relax ordinary-file validation.
            parents.remove(&staged, false).map_err(storage)?;
            if retained_hash(&parents, &live)? != entry.replacement
                || retained_hash(&parents, &held)?.is_some()
            {
                return Err(StorageError::Corrupt);
            }
            continue;
        }
        let current = retained_hash(&parents, &live)?;
        let pending = retained_hash(&parents, &staged)?;
        let original = retained_hash(&parents, &held)?;
        if entry.replacement.is_none() {
            if pending.is_some() {
                return Err(StorageError::Corrupt);
            }
            remove(&parents, &live, &held, entry.original, &mut after_backup)?;
            continue;
        }
        if current == entry.replacement && pending.is_none() {
            if original != entry.original && entry.original != entry.replacement {
                return Err(StorageError::Corrupt);
            }
            continue;
        }
        if pending != entry.replacement {
            return Err(StorageError::Corrupt);
        }
        if current == entry.original && original.is_none() {
            if current.is_some() {
                parents.rename(&live, &held, true).map_err(storage)?;
                commit::sync(live.parent().ok_or(StorageError::UnsafeFile)?)?;
                commit::sync(held.parent().ok_or(StorageError::UnsafeFile)?)?;
                after_backup(BackupBoundary::Published)?;
            }
        } else if current.is_some() || original != entry.original {
            return Err(StorageError::Corrupt);
        }
        parents.rename(&staged, &live, true).map_err(storage)?;
        commit::sync(live.parent().ok_or(StorageError::UnsafeFile)?)?;
        commit::sync(staged.parent().ok_or(StorageError::UnsafeFile)?)?;
    }
    let mut remaining = Vec::new();
    collect(stage, stage, &mut remaining)?;
    if !remaining.is_empty() {
        return Err(StorageError::Corrupt);
    }
    // Release nested Windows sharing guards before deleting the empty staging tree.
    drop(parents);
    drop(preparation);
    let cleanup = Parents::for_owner(root, &[stage], false, owner).map_err(storage)?;
    cleanup.remove_tree(stage).map_err(storage)?;
    commit::sync(work)?;
    Ok(())
}

// Removal has its own durable backup boundary. Recovery accepts the original
// still live or already absent, but never a newly appeared player replacement.
fn remove(
    parents: &Parents,
    live: &Path,
    held: &Path,
    expected: Option<[u8; 32]>,
    after_backup: &mut impl FnMut(BackupBoundary) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    let original = retained_hash(parents, held)?;
    let current = retained_hash(parents, live)?;
    if original.is_none() && current == expected {
        if expected.is_some() {
            let partial = removal_partial(held);
            if retained_hash(parents, &partial)?.is_some() {
                // Only this journal's partial is retryable, and only while the
                // complete original still supplies the immutable expected bytes.
                if retained_hash(parents, live)? != expected {
                    return Err(StorageError::Corrupt);
                }
                parents.remove(&partial, false).map_err(storage)?;
            }
            let mut backup = parents.open(&partial, true, true).map_err(storage)?;
            after_backup(BackupBoundary::Created)?;
            let mut source = parents.open(live, false, false).map_err(storage)?;
            let mut prefix = [0; 65536];
            let count = source.read(&mut prefix).map_err(|_| StorageError::Io)?;
            backup
                .write_all(&prefix[..count])
                .map_err(|_| StorageError::Io)?;
            after_backup(BackupBoundary::PrefixCopied)?;
            std::io::copy(&mut source, &mut backup).map_err(|_| StorageError::Io)?;
            backup
                .sync_all()
                .map_err(|_| StorageError::PersistenceUncertain)?;
            drop(backup);
            if retained_hash(parents, &partial)? != expected
                || retained_hash(parents, live)? != expected
            {
                return Err(StorageError::Corrupt);
            }
            // Publish without replacing a final backup, relative to retained parents.
            parents.rename(&partial, held, true).map_err(storage)?;
            commit::sync(held.parent().ok_or(StorageError::UnsafeFile)?)?;
            parents.remove(&partial, false).map_err(storage)?;
            commit::sync(held.parent().ok_or(StorageError::UnsafeFile)?)?;
            after_backup(BackupBoundary::Published)?;
        }
    } else if original != expected {
        return Err(StorageError::Corrupt);
    }
    let partial = removal_partial(held);
    if retained_hash(parents, &partial)?.is_some()
        && retained_hash(parents, live)? == expected
        && expected.is_some()
    {
        parents.remove(&partial, false).map_err(storage)?;
        commit::sync(held.parent().ok_or(StorageError::UnsafeFile)?)?;
    }
    match retained_hash(parents, live)? {
        Some(current) if Some(current) == expected => {
            parents.remove(live, false).map_err(storage)?;
            commit::sync(live.parent().ok_or(StorageError::UnsafeFile)?)?;
        }
        None => (),
        _ => return Err(StorageError::Corrupt),
    }
    Ok(())
}

fn removal_partial(held: &Path) -> PathBuf {
    let digest = Sha256::digest(held.as_os_str().as_encoded_bytes());
    let digest: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    held.with_file_name(format!(".cimmeria-removal-{digest}.partial"))
}

fn collect(root: &Path, directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), StorageError> {
    for entry in std::fs::read_dir(directory).map_err(|_| StorageError::Io)? {
        let entry = entry.map_err(|_| StorageError::Io)?;
        let metadata = std::fs::symlink_metadata(entry.path()).map_err(|_| StorageError::Io)?;
        ordinary(&metadata)?;
        if metadata.is_dir() {
            collect(root, &entry.path(), paths)?;
        } else if metadata.is_file() {
            paths.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| StorageError::UnsafeFile)?
                    .to_path_buf(),
            );
        } else {
            return Err(StorageError::UnsafeFile);
        }
    }
    Ok(())
}
fn validate_relative(path: &Path) -> Result<(), StorageError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        || path
            .components()
            .next()
            .is_some_and(|c| c.as_os_str().to_string_lossy().starts_with(".cimmeria"))
        || path == Path::new("content-ready.json")
    {
        return Err(StorageError::UnsafeFile);
    }
    Ok(())
}
fn checked(root: &Path, relative: &Path) -> Result<(), StorageError> {
    let mut path = root.to_path_buf();
    for part in relative.components() {
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => ordinary(&metadata)?,
            Err(error) if error.kind() == ErrorKind::NotFound => break,
            Err(_) => return Err(StorageError::Io),
        }
    }
    Ok(())
}
fn hash(path: &Path) -> Result<Option<[u8; 32]>, StorageError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(StorageError::Io),
    };
    ordinary(&metadata)?;
    if !metadata.is_file() {
        return Err(StorageError::UnsafeFile);
    }
    let mut file = File::open(path).map_err(|_| StorageError::Io)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|_| StorageError::Io)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(Some(digest.finalize().into()))
}

mod selection;
#[cfg(test)]
mod tests;
