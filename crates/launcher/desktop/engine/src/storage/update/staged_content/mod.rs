//! Derived integrity receipt for the reconstructed stage, excluding only its
//! operation-specific rename marker. This is not a publisher-signed file index.
use super::*;
use std::{collections::BTreeSet, io::Read};
use tokio_util::sync::CancellationToken;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ready {
    pub plan: Plan,
    pub digest: [u8; 32],
}
pub(super) fn digest(
    tree: &Path,
    plan: &Plan,
    cancel: &CancellationToken,
) -> Result<[u8; 32], StorageError> {
    fn walk(
        root: &Path,
        dir: &Path,
        ignored: &str,
        depth: usize,
        names: &mut BTreeSet<String>,
        sha: &mut Sha256,
        cancel: &CancellationToken,
    ) -> Result<(), StorageError> {
        if depth > 64 {
            return Err(StorageError::TooLarge);
        }
        if !directory_or_absent(dir)? {
            return Err(StorageError::Corrupt);
        }
        let mut entries = std::fs::read_dir(dir)
            .map_err(|_| StorageError::Io)?
            .take(200_001)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StorageError::Io)?;
        if entries.len() > 200_000 {
            return Err(StorageError::TooLarge);
        }
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if cancel.is_cancelled() {
                return Err(StorageError::Busy);
            }
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|_| StorageError::UnsafeFile)?
                .to_str()
                .ok_or(StorageError::UnsafeFile)?
                .replace('\\', "/");
            if relative == ignored {
                continue;
            }
            if relative.len() > 4096
                || names.len() >= 200_000
                || !names.insert(relative.to_ascii_lowercase())
            {
                return Err(StorageError::UnsafeFile);
            }
            let metadata = std::fs::symlink_metadata(&path).map_err(|_| StorageError::Io)?;
            ordinary(&metadata)?;
            sha.update((relative.len() as u64).to_le_bytes());
            sha.update(relative.as_bytes());
            if metadata.is_dir() {
                sha.update(b"d");
                walk(root, &path, ignored, depth + 1, names, sha, cancel)?;
            } else if metadata.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.nlink() != 1 {
                        return Err(StorageError::UnsafeFile);
                    }
                }
                let mut options = OpenOptions::new();
                options.read(true);
                #[cfg(target_os = "macos")]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.custom_flags(libc::O_NOFOLLOW);
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::OpenOptionsExt;
                    options.share_mode(1);
                }
                let mut file = options.open(&path).map_err(|_| StorageError::Io)?;
                let before = file.metadata().map_err(|_| StorageError::Io)?;
                ordinary(&before)?;
                if !before.is_file()
                    || before.len() != metadata.len()
                    || before.modified().ok() != metadata.modified().ok()
                {
                    return Err(StorageError::UnsafeFile);
                }
                sha.update(b"f");
                sha.update(before.len().to_le_bytes());
                let mut content = Sha256::new();
                let mut size = 0;
                let mut buffer = [0; 128 * 1024];
                loop {
                    if cancel.is_cancelled() {
                        return Err(StorageError::Busy);
                    }
                    let n = file.read(&mut buffer).map_err(|_| StorageError::Io)?;
                    if n == 0 {
                        break;
                    }
                    size += n as u64;
                    content.update(&buffer[..n]);
                }
                let after = std::fs::symlink_metadata(&path).map_err(|_| StorageError::Io)?;
                ordinary(&after)?;
                if size != before.len()
                    || after.len() != size
                    || before.modified().ok() != after.modified().ok()
                {
                    return Err(StorageError::UnsafeFile);
                }
                sha.update(content.finalize());
            } else {
                return Err(StorageError::UnsafeFile);
            }
        }
        Ok(())
    }
    let mut sha = Sha256::new();
    walk(
        tree,
        tree,
        &tree_identity::name(plan),
        0,
        &mut BTreeSet::new(),
        &mut sha,
        cancel,
    )?;
    Ok(sha.finalize().into())
}
pub(super) fn verify(state: &DesktopState, plan: &Plan, tree: &Path) -> Result<(), StorageError> {
    let ready: Ready = read(
        &state
            .directory
            .root
            .join(format!("update-prepared-{}.json", plan.id)),
    )?
    .ok_or(StorageError::Corrupt)?;
    if ready.plan != *plan || ready.digest != digest(tree, plan, &CancellationToken::new())? {
        return Err(StorageError::Corrupt);
    }
    Ok(())
}
