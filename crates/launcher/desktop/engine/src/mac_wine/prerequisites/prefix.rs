//! Generation-specific game prefix. Never adopt the extraction prefix or a
//! pre-existing generation, and retain installation/prefix/cache ownership.
use super::*;
use std::io::{Read, Write};
pub(super) struct Resources {
    pub runtime: PathBuf,
    pub prefix: PathBuf,
    pub root: PathBuf,
    _installation: File,
    _prefix: File,
    _cache: File,
}
impl Resources {
    pub fn reopen(plan: &Plan, state_root: &Path) -> Result<Self, IntentError> {
        if hex(&plan.runtime_sha256) != mac_runtime::ARCHIVE_SHA256 {
            return Err(StorageError::Corrupt.into());
        }
        directory(&plan.installation.destination)?;
        let installation =
            lock_owner(&plan.installation.destination.join(".cimmeria-install.json"))?;
        if read_owner::<InstallIntent>(&installation)? != plan.installation {
            return Err(StorageError::Corrupt.into());
        }
        let root = directory(&plan.prefix_directory(state_root))?;
        let owner = lock_owner(&root.join("owner.json"))?;
        if read_owner::<Plan>(&owner)? != *plan {
            return Err(StorageError::Corrupt.into());
        }
        let prefix = directory(&root.join("bottle"))?;
        let (runtime, cache) = mac_runtime::verified_cached(&state_root.join("runtimes"))
            .map_err(|_| StorageError::Io)?;
        Ok(Self {
            runtime,
            prefix,
            root,
            _installation: installation,
            _prefix: owner,
            _cache: cache,
        })
    }
    pub fn claim(plan: &Plan, state_root: &Path, helper: &Path) -> Result<Self, IntentError> {
        if hex(&plan.runtime_sha256) != mac_runtime::ARCHIVE_SHA256 {
            return Err(StorageError::Corrupt.into());
        }
        verify_file(helper, &plan.helper_sha256).map_err(|_| StorageError::UnsafeFile)?;
        let installation = directory(&plan.installation.destination)?;
        let owner_path = installation.join(".cimmeria-install.json");
        let installation_owner = lock_owner(&owner_path)?;
        let saved: InstallIntent = read_owner(&installation_owner)?;
        if saved != plan.installation {
            return Err(StorageError::Corrupt.into());
        }
        // Holds the immutable runtime cache lock for every guest invocation.
        let (runtime, cache) = mac_runtime::verified_cached(&state_root.join("runtimes"))
            .map_err(|_| StorageError::Io)?;
        let parent = state_root.join("game-prefixes");
        create_parent(&parent)?;
        let installation_root = parent.join(plan.installation.operation_id.to_string());
        create_parent(&installation_root)?;
        let root = plan.prefix_directory(state_root);
        std::fs::create_dir(&root).map_err(|_| StorageError::InUse)?;
        let mut marker = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(root.join("owner.json"))
            .map_err(|_| StorageError::Io)?;
        marker.try_lock().map_err(|_| StorageError::InUse)?;
        marker
            .write_all(&serde_json::to_vec(plan).map_err(|_| StorageError::Corrupt)?)
            .map_err(|_| StorageError::Io)?;
        marker.sync_all().map_err(|_| StorageError::Io)?;
        let prefix = root.join("bottle");
        std::fs::create_dir(&prefix).map_err(|_| StorageError::Io)?;
        std::fs::create_dir(prefix.join("drive_c")).map_err(|_| StorageError::Io)?;
        std::fs::create_dir(prefix.join("dosdevices")).map_err(|_| StorageError::Io)?;
        std::os::unix::fs::symlink("../drive_c", prefix.join("dosdevices/c:"))
            .map_err(|_| StorageError::Io)?;
        std::os::unix::fs::symlink("/", prefix.join("dosdevices/z:"))
            .map_err(|_| StorageError::Io)?;
        for path in [&root, &installation_root, &parent] {
            File::open(path)
                .and_then(|f| f.sync_all())
                .map_err(|_| StorageError::Io)?;
        }
        Ok(Self {
            runtime,
            prefix,
            root,
            _installation: installation_owner,
            _prefix: marker,
            _cache: cache,
        })
    }
}
fn create_parent(path: &Path) -> Result<(), StorageError> {
    match std::fs::create_dir(path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(_) => return Err(StorageError::Io),
    }
    directory(path).map(|_| ())
}
pub(super) fn directory(path: &Path) -> Result<PathBuf, StorageError> {
    if !std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
        || path.canonicalize().map_err(|_| StorageError::UnsafeFile)? != path
    {
        return Err(StorageError::UnsafeFile);
    }
    Ok(path.into())
}
pub(super) fn lock_owner(path: &Path) -> Result<File, StorageError> {
    if !std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() <= 65536) {
        return Err(StorageError::UnsafeFile);
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| StorageError::Io)?;
    file.try_lock().map_err(|_| StorageError::InUse)?;
    Ok(file)
}
pub(super) fn read_owner<T: serde::de::DeserializeOwned>(file: &File) -> Result<T, StorageError> {
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| StorageError::Io)?;
    if bytes.len() > 65536 {
        return Err(StorageError::Corrupt);
    }
    serde_json::from_slice(&bytes).map_err(|_| StorageError::Corrupt)
}
