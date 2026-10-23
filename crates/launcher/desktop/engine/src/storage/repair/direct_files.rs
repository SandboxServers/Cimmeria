//! Journaled per-file promotion for adopted roots. Never rename the install tree.
use super::*;
use std::io::Read;

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
) -> Result<(), StorageError> {
    apply_with(plan, root, stage, backup, work, previous, || Ok(()))
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

fn apply_with<P: tree_identity::TreePlan>(
    plan: &P,
    root: &Path,
    stage: &Path,
    backup: &Path,
    work: &Path,
    previous: &[PathBuf],
    mut after_backup: impl FnMut() -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    let saved: Option<Journal<P>> = read(&work.join("direct-files.json"))?;
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
            atomic::write(work, "direct-files.json", &saved)?;
            saved
        }
    };
    if !directory_or_absent(backup)? {
        std::fs::create_dir(backup).map_err(|_| StorageError::Io)?;
        tree_identity::write(backup, plan, tree_identity::Role::Original)?;
    } else if std::fs::read_dir(backup)
        .map_err(|_| StorageError::Io)?
        .next()
        .is_none()
    {
        // Creation can reach disk immediately before its durable marker.
        tree_identity::write(backup, plan, tree_identity::Role::Original)?;
    } else if tree_identity::read_role(backup, plan)? != Some(tree_identity::Role::Original) {
        return Err(StorageError::Corrupt);
    }
    // The immutable journal is complete before any original file is touched.
    for entry in &journal.entries {
        validate_relative(&entry.path)?;
        for tree in [root, stage, backup] {
            checked(tree, &entry.path)?;
        }
        let live = root.join(&entry.path);
        let staged = stage.join(&entry.path);
        let held = backup.join(&entry.path);
        let current = hash(&live)?;
        let pending = hash(&staged)?;
        let original = hash(&held)?;
        if entry.replacement.is_none() {
            if pending.is_some() {
                return Err(StorageError::Corrupt);
            }
            remove(&live, &held, entry.original, &mut after_backup)?;
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
        if entry.original == entry.replacement && current == entry.original && original.is_none() {
            std::fs::remove_file(staged).map_err(|_| StorageError::Io)?;
            continue; // An unchanged file keeps its identity and path.
        }
        if current == entry.original && original.is_none() {
            if current.is_some() {
                std::fs::create_dir_all(held.parent().ok_or(StorageError::UnsafeFile)?)
                    .map_err(|_| StorageError::Io)?;
                std::fs::rename(&live, &held).map_err(|_| StorageError::Io)?;
                commit::sync(live.parent().ok_or(StorageError::UnsafeFile)?)?;
                commit::sync(held.parent().ok_or(StorageError::UnsafeFile)?)?;
                after_backup()?;
            }
        } else if current.is_some() || original != entry.original {
            return Err(StorageError::Corrupt);
        }
        std::fs::create_dir_all(live.parent().ok_or(StorageError::UnsafeFile)?)
            .map_err(|_| StorageError::Io)?;
        std::fs::rename(&staged, &live).map_err(|_| StorageError::Io)?;
        commit::sync(live.parent().ok_or(StorageError::UnsafeFile)?)?;
        commit::sync(staged.parent().ok_or(StorageError::UnsafeFile)?)?;
    }
    if directory_or_absent(stage)? {
        let mut remaining = Vec::new();
        collect(stage, stage, &mut remaining)?;
        if !remaining.is_empty() {
            return Err(StorageError::Corrupt);
        }
        std::fs::remove_dir_all(stage).map_err(|_| StorageError::Io)?;
        commit::sync(work)?;
    }
    Ok(())
}

// Removal has its own durable backup boundary. Recovery accepts the original
// still live or already absent, but never a newly appeared player replacement.
fn remove(
    live: &Path,
    held: &Path,
    expected: Option<[u8; 32]>,
    after_backup: &mut impl FnMut() -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    let original = hash(held)?;
    let current = hash(live)?;
    if original.is_none() && current == expected {
        if expected.is_some() {
            std::fs::create_dir_all(held.parent().ok_or(StorageError::UnsafeFile)?)
                .map_err(|_| StorageError::Io)?;
            let mut backup = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(held)
                .map_err(|_| StorageError::Io)?;
            std::io::copy(
                &mut File::open(live).map_err(|_| StorageError::Io)?,
                &mut backup,
            )
            .map_err(|_| StorageError::Io)?;
            backup
                .sync_all()
                .map_err(|_| StorageError::PersistenceUncertain)?;
            if hash(held)? != expected {
                return Err(StorageError::Corrupt);
            }
            commit::sync(held.parent().ok_or(StorageError::UnsafeFile)?)?;
            after_backup()?;
        }
    } else if original != expected {
        return Err(StorageError::Corrupt);
    }
    match hash(live)? {
        Some(current) if Some(current) == expected => {
            std::fs::remove_file(live).map_err(|_| StorageError::Io)?;
            commit::sync(live.parent().ok_or(StorageError::UnsafeFile)?)?;
        }
        None => (),
        _ => return Err(StorageError::Corrupt),
    }
    Ok(())
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
