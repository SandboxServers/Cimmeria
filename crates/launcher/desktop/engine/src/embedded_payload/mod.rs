//! Trusted executable payloads in an app-local, content-versioned cache.
//! Only compiled entries are accepted; no archive or webview-selected paths.
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
mod paths;
mod sweep;
const OWNER: &[u8] = b"cimmeria-embedded-payload-v1";
/// Fixed Windows resource names; the Mac app retains its bundle resources.
pub const NAMES: [&str; 3] = [
    "cimmeria-launch-worker.exe",
    "cimmeria_client_patches.dll",
    "cimmeria_client_telemetry.dll",
];
/// Bytes and identity generated from the trusted staging inputs at build time.
pub struct Entry {
    pub name: &'static str,
    pub sha256: &'static str,
    pub bytes: &'static [u8],
}
/// Keep this alive while native launch resources can be used.
pub struct Lease {
    root: PathBuf,
    _lock: Lock,
    _directories: Vec<File>,
}
impl Lease {
    /// Canonical resource root, with the existing windows/ layout.
    pub fn root(&self) -> &Path {
        &self.root
    }
}
struct Lock(File);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn invalid() -> io::Error {
    io::Error::other("launcher payload cache refused")
}
fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}
fn identity(entries: &[Entry]) -> io::Result<String> {
    let mut digest = Sha256::new();
    let mut seen = std::collections::BTreeSet::new();
    for entry in entries {
        if !NAMES.contains(&entry.name)
            || !seen.insert(entry.name)
            || entry.bytes.len() > 128 * 1024 * 1024
            || hex(Sha256::digest(entry.bytes)) != entry.sha256
        {
            return Err(invalid());
        }
        digest.update(entry.name.as_bytes());
        digest.update([0]);
        digest.update(entry.sha256.as_bytes());
    }
    if entries.is_empty() {
        return Err(invalid());
    }
    Ok(hex(digest.finalize()))
}
/// Verify/recreate compiled resources. No data outside this dedicated cache is
/// touched. A shared lease prevents cooperating launchers deleting active bytes.
pub fn prepare(cache: &Path, entries: &[Entry]) -> io::Result<Lease> {
    let result = prepare_inner(cache, entries);
    if result.is_err() {
        tracing::warn!(target: "launcher.startup", event = "payload_cache_refused",
            reason = "prepare_failed", "launcher payload could not be prepared; startup stops");
    }
    result
}
fn prepare_inner(cache: &Path, entries: &[Entry]) -> io::Result<Lease> {
    let id = identity(entries)?;
    let mut directories = paths::create_tree(cache)?;
    let cache = cache.canonicalize()?;
    let global = Lock(paths::open_file(&cache.join(".cache-lock"))?);
    global.0.lock()?;
    let root = cache.join(&id);
    directories.push(paths::directory(&root, true)?);
    let root = root.canonicalize()?;
    let lease = Lock(paths::open_file(&root.join(".lease"))?);
    let owner = root.join(".owner");
    let owned = match fs::symlink_metadata(&owner) {
        Ok(_) => {
            paths::plain_file(&owner)?;
            if fs::metadata(&owner)?.len() != OWNER.len() as u64 || fs::read(&owner)? != OWNER {
                return Err(invalid());
            }
            true
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };
    let windows = root.join("windows");
    directories.push(paths::directory(&windows, true)?);
    let valid = entries
        .iter()
        .all(|entry| matches_entry(&windows.join(entry.name), entry));
    if !valid {
        // Never replace files used by another launcher. In-use Windows images
        // are also protected by the OS after a launcher exits but its game stays.
        lease.0.try_lock().map_err(|_| invalid())?;
        for entry in entries {
            let path = windows.join(entry.name);
            if !matches_entry(&path, entry) {
                paths::replace(&path, entry.bytes)?;
            }
        }
    }
    for entry in entries {
        if !matches_entry(&windows.join(entry.name), entry) {
            return Err(invalid());
        }
    }
    if !owned {
        paths::replace(&owner, OWNER)?;
    }
    // lock_shared is held until Lease drops, not just until startup finishes.
    if !valid {
        lease.0.unlock()?;
    }
    lease.0.lock_shared()?;
    let removed = sweep::unused(&cache, &id);
    tracing::info!(target: "launcher.startup", event = "payload_cache_ready",
        payload_sha256 = id, files = entries.len(), reused = valid, removed_versions = removed,
        "launcher embedded payload verified");
    Ok(Lease {
        root,
        _lock: lease,
        _directories: directories,
    })
}
fn matches_entry(path: &Path, entry: &Entry) -> bool {
    paths::plain_file(path).is_ok()
        && File::open(path)
            .and_then(|file| {
                let mut bytes = Vec::new();
                file.take(entry.bytes.len() as u64 + 1)
                    .read_to_end(&mut bytes)?;
                Ok(bytes)
            })
            .is_ok_and(|bytes| {
                bytes.len() == entry.bytes.len() && hex(Sha256::digest(&bytes)) == entry.sha256
            })
}
#[cfg(test)]
mod tests;
