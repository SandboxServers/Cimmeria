//! Canonical digest of every runtime path, type, execute bits and file/link data.
//! Identity is pinned from the authenticated upstream archive, not a local receipt.
use super::*;
use std::os::unix::fs::PermissionsExt;
fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}
pub(super) fn digest(root: &Path, cancel: &CancellationToken) -> Result<String, RuntimeError> {
    if !std::fs::symlink_metadata(root)?.is_dir() {
        return Err(RuntimeError::Verification);
    }
    let mut entries = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        cancelled(cancel)?;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path)?;
            if meta.is_dir() {
                pending.push(path.clone());
            }
            entries.push((path, meta));
            if entries.len() > 5000 {
                return Err(RuntimeError::Verification);
            }
        }
    }
    entries.sort_by(|a, b| {
        a.0.as_os_str()
            .as_encoded_bytes()
            .cmp(b.0.as_os_str().as_encoded_bytes())
    });
    let mut hash = Sha256::new();
    let mut size = 0u64;
    for (path, meta) in entries {
        cancelled(cancel)?;
        let relative = path
            .strip_prefix(root)
            .map_err(|_| RuntimeError::Verification)?;
        field(
            &mut hash,
            relative
                .to_str()
                .ok_or(RuntimeError::Verification)?
                .as_bytes(),
        );
        let kind = if meta.is_dir() {
            b'd'
        } else if meta.file_type().is_symlink() {
            b'l'
        } else if meta.is_file() {
            b'f'
        } else {
            return Err(RuntimeError::Verification);
        };
        hash.update([kind]);
        hash.update((meta.permissions().mode() & 0o111).to_le_bytes());
        if kind == b'l' {
            let link = std::fs::read_link(&path)?;
            field(
                &mut hash,
                link.to_str().ok_or(RuntimeError::Verification)?.as_bytes(),
            );
        } else if kind == b'f' {
            size = size
                .checked_add(meta.len())
                .ok_or(RuntimeError::Verification)?;
            if size > 1_000_000_000 {
                return Err(RuntimeError::Verification);
            }
            let mut file = File::open(path)?;
            let mut digest = Sha256::new();
            let mut bytes = [0u8; 65536];
            loop {
                cancelled(cancel)?;
                let n = file.read(&mut bytes)?;
                if n == 0 {
                    break;
                }
                digest.update(&bytes[..n]);
            }
            hash.update(digest.finalize());
        }
    }
    Ok(hex(&hash.finalize()))
}
pub(super) fn verify(
    root: &Path,
    expected: &str,
    cancel: &CancellationToken,
) -> Result<(), RuntimeError> {
    if digest(root, cancel)? != expected {
        return Err(RuntimeError::Verification);
    }
    Ok(())
}
