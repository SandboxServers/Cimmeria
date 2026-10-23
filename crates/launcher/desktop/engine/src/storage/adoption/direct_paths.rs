//! Retained ordinary-directory handles for direct-root content mutations.
//! Windows denies parent deletion; Unix mutations use the retained parent fd.
use super::*;
use std::collections::BTreeMap;
#[path = "direct_path_ops.rs"]
mod operations;

pub(crate) struct Parents {
    root: PathBuf,
    directories: BTreeMap<PathBuf, File>,
}
impl Parents {
    pub(super) fn retain_root(root: &Path) -> Result<Self, Error> {
        let file = open_directory(root)?;
        Ok(Self {
            root: root.to_owned(),
            directories: [(PathBuf::new(), file)].into(),
        })
    }
    pub(super) fn new(plan: &Plan, paths: &[&Path], create: bool) -> Result<Self, Error> {
        let root = &plan.installation.destination;
        inventory::same_directory(root, &plan.destination_identity)?;
        let mut result = Self::for_root(root, &[], false)?;
        let retained = result
            .directories
            .get(Path::new(""))
            .ok_or(StorageError::UnsafeFile)?;
        let actual = inventory::file_identity(retained)?;
        if actual.device != plan.destination_identity.device
            || actual.inode != plan.destination_identity.inode
        {
            return Err(StorageError::UnsafeFile.into());
        }
        result.retain_paths(paths, create)?;
        Ok(result)
    }
    /// Retain an ordinary root and each mutation parent; callers authenticate root ownership.
    pub(crate) fn for_root(root: &Path, paths: &[&Path], create: bool) -> Result<Self, Error> {
        let expected = inventory::identity(root)?;
        let mut result = Self::retain_root(root)?;
        let retained = result
            .directories
            .get(Path::new(""))
            .ok_or(StorageError::UnsafeFile)?;
        let actual = inventory::file_identity(retained)?;
        if actual.device != expected.device || actual.inode != expected.inode {
            return Err(StorageError::UnsafeFile.into());
        }
        inventory::same_directory(root, &expected)?;
        result.retain_paths(paths, create)?;
        Ok(result)
    }
    fn retain_paths(&mut self, paths: &[&Path], create: bool) -> Result<(), Error> {
        for path in paths {
            let relative = path
                .strip_prefix(&self.root)
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
                if !self.directories.contains_key(&child) {
                    let directory = self.open_child_directory(&parent, part.as_os_str(), create)?;
                    let Some(directory) = directory else { break };
                    self.directories.insert(child.clone(), directory);
                }
                parent = child;
            }
        }
        Ok(())
    }
    #[cfg(not(unix))]
    fn child(&self, parent: &Path, name: &std::ffi::OsStr) -> Result<PathBuf, Error> {
        self.directories
            .get(parent)
            .ok_or(StorageError::UnsafeFile)?;
        Ok(self.root.join(parent).join(name))
    }
    #[cfg(not(unix))]
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
        options.custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_dir() || !super::super::failed_cleanup::plain(&meta) {
        return Err(StorageError::UnsafeFile.into());
    }
    Ok(file)
}

pub(super) fn remove(plan: &Plan, path: &Path) -> Result<(), Error> {
    let parents = Parents::new(plan, &[path], false)?;
    parents.remove(path, false)
}
pub(super) fn copy(plan: &Plan, from: &Path, to: &Path) -> Result<(), Error> {
    let parents = Parents::new(plan, &[from, to], true)?;
    parents.copy_atomic(from, to)
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

/// Publish a verified sibling without replacing any concurrently created target.
#[cfg(windows)]
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
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn plan(root: &Path) -> Plan {
        let mut plan: Plan =
            serde_json::from_str(include_str!("fixtures/plan-v1-in-place.json")).unwrap();
        plan.schema_version = 2;
        plan.installation.schema_version = 2;
        plan.installation.destination = root.canonicalize().unwrap();
        plan.destination_identity = inventory::identity(&plan.installation.destination).unwrap();
        plan
    }
    #[test]
    #[cfg(windows)]
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
        assert_eq!(guard.read(&target).unwrap(), b"owned bytes");
        let displaced = plan.installation.destination.join("displaced");
        #[cfg(windows)]
        assert!(std::fs::rename(target.parent().unwrap(), &displaced).is_err());
        #[cfg(unix)]
        {
            std::fs::rename(target.parent().unwrap(), &displaced).unwrap();
            let outside = tempfile::tempdir().unwrap();
            std::fs::write(outside.path().join("file.txt"), b"outside").unwrap();
            std::os::unix::fs::symlink(outside.path(), target.parent().unwrap()).unwrap();
            assert_eq!(guard.read(&target).unwrap(), b"owned bytes");
            assert!(Parents::new(&plan, &[&target], false).is_err());
        }
    }
}

#[cfg(all(test, unix))]
#[path = "direct_path_tests.rs"]
mod unix_tests;
