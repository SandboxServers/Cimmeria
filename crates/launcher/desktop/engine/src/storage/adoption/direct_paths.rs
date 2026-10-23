//! Retained ordinary-directory handles for direct-root content mutations.
//! Windows denies parent deletion; Unix mutations use the retained parent fd.
use super::*;
use std::collections::BTreeMap;

pub(super) struct Parents {
    root: PathBuf,
    directories: BTreeMap<PathBuf, File>,
}
impl Parents {
    pub(super) fn new(plan: &Plan, paths: &[&Path], create: bool) -> Result<Self, Error> {
        let root = &plan.installation.destination;
        inventory::same_directory(root, &plan.destination_identity)?;
        let mut result = Self {
            root: root.clone(),
            directories: BTreeMap::new(),
        };
        result
            .directories
            .insert(PathBuf::new(), open_directory(root)?);
        inventory::same_directory(root, &plan.destination_identity)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let meta = result
                .directories
                .get(Path::new(""))
                .ok_or(StorageError::UnsafeFile)?
                .metadata()?;
            if meta.dev() != plan.destination_identity.device
                || meta.ino() != plan.destination_identity.inode
            {
                return Err(StorageError::UnsafeFile.into());
            }
        }
        for path in paths {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| StorageError::UnsafeFile)?;
            if relative
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err(StorageError::UnsafeFile.into());
            }
            let mut parent = PathBuf::new();
            for part in relative.parent().unwrap_or(Path::new("")).components() {
                let child = parent.join(part);
                if !result.directories.contains_key(&child) {
                    let anchored = result.child(&parent, part.as_os_str())?;
                    match std::fs::symlink_metadata(&anchored) {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound && create => {
                            std::fs::create_dir(&anchored)?;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                        Err(error) => return Err(error.into()),
                        Ok(_) => (),
                    }
                    result
                        .directories
                        .insert(child.clone(), open_directory(&anchored)?);
                }
                parent = child;
            }
        }
        Ok(result)
    }
    fn child(&self, parent: &Path, name: &std::ffi::OsStr) -> Result<PathBuf, Error> {
        let file = self
            .directories
            .get(parent)
            .ok_or(StorageError::UnsafeFile)?;
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = file.metadata()?;
            let anchor = PathBuf::from(format!("/.vol/{}/{}", metadata.dev(), metadata.ino()));
            // Darwin volfs resolves the retained directory identity, rather than
            // a replaceable pathname. Keep the handle alive against inode reuse.
            let actual = std::fs::metadata(&anchor)?;
            if actual.dev() != metadata.dev() || actual.ino() != metadata.ino() {
                return Err(StorageError::UnsafeFile.into());
            }
            Ok(anchor.join(name))
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            use std::os::fd::AsRawFd;
            Ok(PathBuf::from(format!("/dev/fd/{}", file.as_raw_fd())).join(name))
        }
        #[cfg(not(unix))]
        {
            let _ = file;
            Ok(self.root.join(parent).join(name))
        }
    }
    pub(super) fn path(&self, path: &Path) -> Result<PathBuf, Error> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| StorageError::UnsafeFile)?;
        self.child(
            relative.parent().unwrap_or(Path::new("")),
            relative.file_name().ok_or(StorageError::UnsafeFile)?,
        )
    }
}
fn open_directory(path: &Path) -> Result<File, Error> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::*;
        options
            .access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // O_NOFOLLOW closes the gap between inspecting and opening a parent.
        #[cfg(target_os = "macos")]
        options.custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW);
        #[cfg(not(target_os = "macos"))]
        options.custom_flags(0x10000 | 0x20000);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_dir() || !super::super::failed_cleanup::plain(&meta) {
        return Err(StorageError::UnsafeFile.into());
    }
    Ok(file)
}

pub(super) fn remove(plan: &Plan, path: &Path) -> Result<(), Error> {
    if matches!(std::fs::symlink_metadata(path), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(());
    }
    let parents = Parents::new(plan, &[path], false)?;
    let path = parents.path(path)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
pub(super) fn copy(plan: &Plan, from: &Path, to: &Path) -> Result<(), Error> {
    let parents = Parents::new(plan, &[from, to], true)?;
    let (from, to) = (parents.path(from)?, parents.path(to)?);
    let entry = inventory::hash(&from, &CancellationToken::new())?;
    // A verified sibling copy, followed by rename, never truncates a live original.
    let mut partial = to.as_os_str().to_owned();
    partial.push(".adopt-partial");
    let partial = PathBuf::from(partial);
    if super::in_place::present(&partial)? {
        std::fs::remove_file(&partial)?;
    }
    inventory::copy(&from, &partial, &entry, &CancellationToken::new())?;
    std::fs::rename(&partial, &to)?;
    Ok(())
}

/// Preflight all reviewed live paths before a recovery or abandonment can mutate.
pub(super) fn validate_plan(plan: &Plan) -> Result<(), Error> {
    let root = &plan.installation.destination;
    let paths: Vec<PathBuf> = plan
        .report
        .files
        .iter()
        .filter(|row| row.classification != Classification::Extra)
        .map(|row| inventory::native(root, row.source_path.as_deref().unwrap_or(&row.path)))
        .collect();
    let borrowed: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    Parents::new(plan, &borrowed, false)?;
    Ok(())
}

/// Keep the executable entry pinned while setup mutates its header.
pub(super) fn edit_file(
    path: &Path,
    edit: impl FnOnce(&Path) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::*;
        options
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(target_os = "macos")]
        options.custom_flags(libc::O_NOFOLLOW);
        #[cfg(not(target_os = "macos"))]
        options.custom_flags(0x20000);
    }
    let held = options.open(path)?;
    let metadata = held.metadata()?;
    if !metadata.is_file() || !super::super::failed_cleanup::plain(&metadata) {
        return Err(StorageError::UnsafeFile.into());
    }
    #[cfg(windows)]
    {
        inventory::hash(path, &CancellationToken::new())?;
        edit(path)
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(StorageError::UnsafeFile.into());
        }
        #[cfg(target_os = "macos")]
        let pinned = PathBuf::from(format!("/.vol/{}/{}", metadata.dev(), metadata.ino()));
        #[cfg(not(target_os = "macos"))]
        let pinned = {
            use std::os::fd::AsRawFd;
            PathBuf::from(format!("/dev/fd/{}", held.as_raw_fd()))
        };
        edit(&pinned)
    }
    #[cfg(not(any(unix, windows)))]
    Err(StorageError::UnsafeFile.into())
}

/// Publish a verified sibling without replacing any concurrently created target.
pub(super) fn promote_new(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: both UTF-16 strings are owned, NUL terminated and live through the call.
        let result = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        };
        if result == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let from = std::ffi::CString::new(from.as_os_str().as_bytes())?;
        let to = std::ffi::CString::new(to.as_os_str().as_bytes())?;
        #[cfg(target_os = "macos")]
        // SAFETY: valid NUL terminated paths; RENAME_EXCL is the atomic no-replace operation.
        let result = unsafe {
            libc::renameatx_np(
                libc::AT_FDCWD,
                from.as_ptr(),
                libc::AT_FDCWD,
                to.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        #[cfg(not(target_os = "macos"))]
        let result = {
            unsafe extern "C" {
                fn renameat2(
                    oldfd: i32,
                    old: *const std::ffi::c_char,
                    newfd: i32,
                    new: *const std::ffi::c_char,
                    flags: u32,
                ) -> i32;
            }
            // SAFETY: valid NUL terminated paths; Linux RENAME_NOREPLACE is flag 1.
            unsafe { renameat2(-100, from.as_ptr(), -100, to.as_ptr(), 1) }
        };
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    Err(std::io::ErrorKind::Unsupported.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plan(root: &Path) -> Plan {
        let mut plan: Plan =
            serde_json::from_str(include_str!("fixtures/plan-v1-in-place.json")).unwrap();
        plan.schema_version = 2;
        plan.installation.schema_version = 2;
        plan.installation.destination = root.canonicalize().unwrap();
        plan.destination_identity = inventory::identity(&plan.installation.destination).unwrap();
        plan
    }
    #[test]
    fn exclusive_promotion_preserves_a_concurrently_created_target() {
        let root = tempfile::tempdir().unwrap();
        let staged = root.path().join("partial");
        let target = root.path().join("player.txt");
        std::fs::write(&staged, b"release bytes").unwrap();
        std::fs::write(&target, b"new player edit").unwrap();
        assert_eq!(
            promote_new(&staged, &target).unwrap_err().kind(),
            std::io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"new player edit");
        assert_eq!(std::fs::read(&staged).unwrap(), b"release bytes");
    }
    #[test]
    fn directory_handle_paths_reach_the_original_files() {
        let root = tempfile::tempdir().unwrap();
        let plan = plan(root.path());
        let target = plan.installation.destination.join("nested/file.txt");
        std::fs::create_dir(target.parent().unwrap()).unwrap();
        std::fs::write(&target, b"owned bytes").unwrap();
        let guard = Parents::new(&plan, &[&target], false).unwrap();
        assert_eq!(
            std::fs::read(guard.path(&target).unwrap()).unwrap(),
            b"owned bytes"
        );
        let displaced = plan.installation.destination.join("displaced");
        #[cfg(windows)]
        assert!(std::fs::rename(target.parent().unwrap(), &displaced).is_err());
        #[cfg(unix)]
        {
            std::fs::rename(target.parent().unwrap(), &displaced).unwrap();
            let outside = tempfile::tempdir().unwrap();
            std::fs::write(outside.path().join("file.txt"), b"outside").unwrap();
            std::os::unix::fs::symlink(outside.path(), target.parent().unwrap()).unwrap();
            assert_eq!(
                std::fs::read(guard.path(&target).unwrap()).unwrap(),
                b"owned bytes"
            );
            assert!(Parents::new(&plan, &[&target], false).is_err());
        }
    }
}
