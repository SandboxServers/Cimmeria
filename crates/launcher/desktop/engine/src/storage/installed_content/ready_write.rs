//! Receipt publication bound to the authenticated direct-root owner handle.
use super::*;
use crate::storage::adoption::direct_paths::Parents;
use std::io::Write;

fn storage(error: adoption::Error) -> StorageError {
    match error {
        adoption::Error::Storage(error) => error,
        _ => StorageError::UnsafeFile,
    }
}

pub(crate) fn write_ready_owned(
    root: &Path,
    owner: &InstallIntent,
    release: ReleaseIdentity,
    owner_file: &File,
) -> Result<(), StorageError> {
    if owner.schema_version != 2 {
        return write_ready(root, owner, release);
    }
    let parents = Parents::for_owner(root, &[], false, owner_file).map_err(storage)?;
    publish(&parents, root, owner, release)
}

fn publish(
    parents: &Parents,
    root: &Path,
    owner: &InstallIntent,
    release: ReleaseIdentity,
) -> Result<(), StorageError> {
    let bytes = if release == owner.release_identity() {
        serde_json::to_vec(owner)
    } else {
        serde_json::to_vec(&ContentReceipt {
            schema_version: 2,
            installation_id: owner.operation_id,
            release,
        })
    }
    .map_err(|_| StorageError::Corrupt)?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(StorageError::TooLarge);
    }
    let temporary = root.join(format!(".launcher-state-{}.partial", uuid::Uuid::new_v4()));
    let mut file = parents.open(&temporary, true, true).map_err(storage)?;
    file.write_all(&bytes).map_err(|_| StorageError::Io)?;
    file.sync_all().map_err(|_| StorageError::Io)?;
    drop(file);
    // The rename and Unix parent sync both use retained directory descriptors.
    parents
        .rename(&temporary, &root.join("content-ready.json"), false)
        .map_err(|_| StorageError::PersistenceUncertain)
}

#[cfg(test)]
mod tests;
