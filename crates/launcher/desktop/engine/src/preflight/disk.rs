//! The disk and game-folder probes preflight reads (C-PD4, PD-07).

use std::path::{Path, PathBuf};

use super::FolderFacts;

/// Free bytes available to this user on the volume holding `path`, measured at
/// its nearest existing ancestor directory. Any error gives `None`.
pub fn free_bytes(path: &Path) -> Option<u64> {
    let dir = nearest_existing_dir(path)?;
    volume_free_bytes(&dir)
}

/// What the launcher knows about the game folder at `path`.
///
/// `writable` is a real probe: a `.cimmeria-write-probe-<uuid>` file is created
/// and removed in the folder or, when `!must_exist` and the folder is absent,
/// in its nearest existing ancestor (where the install would create it).
pub fn folder_facts(path: Option<&Path>, must_exist: bool) -> FolderFacts {
    let Some(path) = path else {
        return FolderFacts {
            chosen: false,
            exists: false,
            writable: false,
        };
    };
    let exists = path.is_dir();
    let writable = if exists {
        write_probe(path)
    } else if !must_exist && std::fs::symlink_metadata(path).is_err() {
        nearest_existing_dir(path).is_some_and(|dir| write_probe(&dir))
    } else {
        false
    };
    FolderFacts {
        chosen: true,
        exists,
        writable,
    }
}

/// The path itself or its closest ancestor that is an existing directory.
fn nearest_existing_dir(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .map(|ancestor| {
            if ancestor.as_os_str().is_empty() {
                Path::new(".")
            } else {
                ancestor
            }
        })
        .find(|ancestor| ancestor.is_dir())
        .map(Path::to_path_buf)
}

/// Creates and removes a uniquely named file in `dir`; true when both succeed.
fn write_probe(dir: &Path) -> bool {
    let probe = dir.join(format!(".cimmeria-write-probe-{}", uuid::Uuid::new_v4()));
    let created = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe);
    match created {
        Ok(file) => {
            drop(file);
            std::fs::remove_file(&probe).is_ok()
        }
        Err(_) => false,
    }
}

#[cfg(unix)]
fn volume_free_bytes(dir: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;

    let c_path = std::ffi::CString::new(dir.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::zeroed();
    // SAFETY: `c_path` is a NUL-terminated string that outlives the call and
    // `stat` points to writable memory sized for one `statvfs`.
    let result = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return None;
    }
    // SAFETY: `statvfs` returned 0, so it filled `stat`.
    let stat = unsafe { stat.assume_init() };
    // The field widths differ between targets (32-bit block counts on macOS).
    #[allow(clippy::unnecessary_cast)]
    let (available, fragment) = (stat.f_bavail as u64, stat.f_frsize as u64);
    available.checked_mul(fragment)
}

#[cfg(windows)]
fn volume_free_bytes(dir: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available: u64 = 0;
    // SAFETY: `wide` is NUL-terminated and outlives the call; the out pointer is
    // a valid `u64` and the two optional totals are null, which the API allows.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(available)
}

#[cfg(not(any(unix, windows)))]
fn volume_free_bytes(_dir: &Path) -> Option<u64> {
    None
}
