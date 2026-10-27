//! Remove only recognized, owned and unleased payload files; never recurse.
use super::*;
pub(super) fn unused(cache: &Path, current: &str) -> usize {
    let Ok(entries) = fs::read_dir(cache) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name == current || name.len() != 64 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        if remove(&entry.path()).is_ok() {
            removed += 1;
        } else {
            tracing::info!(target: "launcher.startup", event = "payload_cache_retained",
                payload_sha256 = name, reason = "cleanup_deferred",
                "launcher payload cache version retained for a later start");
        }
    }
    removed
}
fn remove(root: &Path) -> io::Result<()> {
    let directory = paths::directory(root, false)?;
    let owner = root.join(".owner");
    paths::plain_file(&owner)?;
    if fs::metadata(&owner)?.len() != OWNER.len() as u64 || fs::read(&owner)? != OWNER {
        return Err(invalid());
    }
    paths::plain_file(&root.join(".lease"))?;
    let lease = Lock(paths::open_file(&root.join(".lease"))?);
    lease.0.try_lock().map_err(|_| invalid())?;
    // Refuse the entire version before deleting anything if unknown content
    // appeared. A name, even a digest-shaped one, never authorizes recursion.
    for entry in fs::read_dir(root)? {
        let name = entry?.file_name();
        if ![".owner", ".lease", "windows"]
            .iter()
            .any(|known| name == *known)
        {
            return Err(invalid());
        }
    }
    let windows = root.join("windows");
    let windows_directory = paths::directory(&windows, false)?;
    let mut files = Vec::new();
    for entry in fs::read_dir(&windows)? {
        let entry = entry?;
        if !entry.file_name().to_str().is_some_and(is_payload_name) {
            return Err(invalid());
        }
        paths::plain_file(&entry.path())?;
        files.push(entry.path());
    }
    for path in files {
        fs::remove_file(path)?;
    }
    drop(windows_directory);
    fs::remove_dir(windows)?;
    fs::remove_file(owner)?;
    drop(lease);
    fs::remove_file(root.join(".lease"))?;
    drop(directory);
    fs::remove_dir(root)
}
