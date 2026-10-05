use super::{ensure_regular_or_absent, StorageError, MAX_STATE_BYTES};
use serde::Serialize;
use std::{io::Write, path::Path};

pub(super) fn write<T: Serialize>(root: &Path, name: &str, value: &T) -> Result<(), StorageError> {
    write_with(root, name, value, |_| Ok(()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Checkpoint {
    BeforeReplace,
    AfterReplace,
}

// The checkpoint hook is private and used for deterministic filesystem fault tests.
pub(super) fn write_with<T: Serialize>(
    root: &Path,
    name: &str,
    value: &T,
    checkpoint: impl FnMut(Checkpoint) -> std::io::Result<()>,
) -> Result<(), StorageError> {
    let bytes = serde_json::to_vec(value).map_err(|_| StorageError::Corrupt)?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(StorageError::TooLarge);
    }
    replace(root, name, &bytes, checkpoint)
}

pub(super) fn write_bytes(
    root: &Path,
    name: &str,
    bytes: &[u8],
    limit: usize,
) -> Result<(), StorageError> {
    if bytes.len() > limit {
        return Err(StorageError::TooLarge);
    }
    replace(root, name, bytes, |_| Ok(()))
}

fn replace(
    root: &Path,
    name: &str,
    bytes: &[u8],
    mut checkpoint: impl FnMut(Checkpoint) -> std::io::Result<()>,
) -> Result<(), StorageError> {
    let path = root.join(name);
    ensure_regular_or_absent(&path)?;
    let mut temp = tempfile::Builder::new()
        .prefix(".launcher-state-")
        .tempfile_in(root)
        .map_err(|_| StorageError::Io)?;
    temp.write_all(bytes).map_err(|_| StorageError::Io)?;
    temp.as_file().sync_all().map_err(|_| StorageError::Io)?;
    checkpoint(Checkpoint::BeforeReplace).map_err(|_| StorageError::Io)?;
    let file = temp.persist(&path).map_err(|_| StorageError::Io)?;
    // Once replacement succeeds, any subsequent failure is an uncertain commit.
    checkpoint(Checkpoint::AfterReplace).map_err(|_| StorageError::PersistenceUncertain)?;
    file.sync_all()
        .map_err(|_| StorageError::PersistenceUncertain)?;
    #[cfg(unix)]
    std::fs::File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| StorageError::PersistenceUncertain)?;
    // Windows file replacement is atomic via tempfile. Power-loss durability of
    // directory metadata needs Windows-native validation; never claim it here.
    Ok(())
}
