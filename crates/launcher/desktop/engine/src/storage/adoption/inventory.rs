//! Bounded deterministic filesystem evidence. Source file handles never write.
use super::*;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};

pub(super) const MAX_FILES: usize = 200_000;
pub(super) const MAX_DEPTH: usize = 64;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub(super) device: u64,
    pub(super) inode: u64,
    length: u64,
    modified_ns: u128,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Entry {
    pub sha256: [u8; 32],
    pub size: u64,
    pub identity: Identity,
}
pub(super) type Index = BTreeMap<String, Entry>;
/// The on-disk path of an index name. Names always use `/`; a verbatim
/// (`\\?\`) Windows root takes separators literally, so join per component.
pub(super) fn native(root: &Path, name: &str) -> PathBuf {
    name.split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}
#[cfg(unix)]
pub(super) fn identity(path: &Path) -> Result<Identity, Error> {
    let metadata = std::fs::symlink_metadata(path)?;
    identity_of(&metadata)
}
#[cfg(unix)]
fn file_identity(file: &File) -> Result<Identity, Error> {
    identity_of(&file.metadata()?)
}
/// Volume serial and file index stand in for device and inode. The handle is
/// opened on the entry itself, never through a link or junction.
#[cfg(windows)]
pub(super) fn identity(path: &Path) -> Result<Identity, Error> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0 {
        return Err(StorageError::UnsafeFile.into());
    }
    let file = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    file_identity(&file)
}
#[cfg(windows)]
fn file_identity(file: &File) -> Result<Identity, Error> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT,
    };
    // SAFETY: plain-old-data out parameter, filled by the call below.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: the handle is owned by `file` and stays open for the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let directory = info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0;
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (!directory && info.nNumberOfLinks != 1)
    {
        return Err(StorageError::UnsafeFile.into());
    }
    let wide = |high: u32, low: u32| (u64::from(high) << 32) | u64::from(low);
    Ok(Identity {
        device: u64::from(info.dwVolumeSerialNumber),
        inode: wide(info.nFileIndexHigh, info.nFileIndexLow),
        length: if directory {
            0
        } else {
            wide(info.nFileSizeHigh, info.nFileSizeLow)
        },
        // FILETIME counts 100 ns intervals.
        modified_ns: u128::from(wide(
            info.ftLastWriteTime.dwHighDateTime,
            info.ftLastWriteTime.dwLowDateTime,
        )) * 100,
    })
}
#[cfg(not(any(unix, windows)))]
pub(super) fn identity(_: &Path) -> Result<Identity, Error> {
    Err(StorageError::UnsafeFile.into())
}
#[cfg(not(any(unix, windows)))]
fn file_identity(_: &File) -> Result<Identity, Error> {
    Err(StorageError::UnsafeFile.into())
}
#[cfg(unix)]
fn identity_of(metadata: &std::fs::Metadata) -> Result<Identity, Error> {
    if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
        return Err(StorageError::UnsafeFile.into());
    }
    use std::os::unix::fs::MetadataExt;
    if metadata.is_file() && metadata.nlink() != 1 {
        return Err(StorageError::UnsafeFile.into());
    }
    Ok(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
        length: metadata.len(),
        modified_ns: metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| StorageError::UnsafeFile)?
            .as_nanos(),
    })
}
/// The scan's rule for a name (D-LF3): any valid-Unicode name with no control
/// character, `:` or `\`, at most 4,096 bytes.
pub(super) fn scan_name_allowed(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 4096
        && !name.contains(':')
        && !name.contains('\\')
        && !name.chars().any(char::is_control)
}
/// Why `path` itself cannot be scanned, or `None` when it can. Read only: a
/// Windows handle is opened with FILE_READ_ATTRIBUTES and
/// FILE_FLAG_OPEN_REPARSE_POINT, so a cloud file is never recalled.
pub(super) fn unsafe_reason(path: &Path) -> Option<UnsafeReason> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0 {
            return Some(reparse_tag(path).map_or(UnsafeReason::Link, reparse_reason));
        }
    }
    if metadata.file_type().is_symlink() {
        return Some(UnsafeReason::Link);
    }
    if !metadata.is_file() && !metadata.is_dir() {
        return Some(UnsafeReason::SpecialFile);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.is_file() && metadata.nlink() != 1 {
            return Some(UnsafeReason::HardLink);
        }
    }
    match identity(path) {
        // On Windows, once reparse points are ruled out, `file_identity` refuses
        // a plain file only for its link count.
        #[cfg(windows)]
        Err(Error::Storage(StorageError::UnsafeFile)) if metadata.is_file() => {
            Some(UnsafeReason::HardLink)
        }
        Err(Error::Storage(StorageError::UnsafeFile)) => Some(UnsafeReason::SpecialFile),
        _ => None,
    }
}
/// The reason a Windows reparse tag stands for (D-AU6). Pure, tested on every host.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn reparse_reason(tag: u32) -> UnsafeReason {
    const MOUNT_POINT: u32 = 0xA000_0003;
    const SYMLINK: u32 = 0xA000_000C;
    // IO_REPARSE_TAG_CLOUD .. CLOUD_F differ only in bits 12-15.
    const CLOUD_MASK: u32 = 0xFFFF_0FFF;
    const CLOUD: u32 = 0x9000_001A;
    const ONEDRIVE: u32 = 0x8000_0021;
    match tag {
        MOUNT_POINT | SYMLINK => UnsafeReason::Link,
        tag if tag & CLOUD_MASK == CLOUD || tag == ONEDRIVE => UnsafeReason::CloudFile,
        _ => UnsafeReason::SpecialFile,
    }
}
/// The reparse tag of `path` itself, read without following it. `None` when it
/// cannot be read.
#[cfg(windows)]
fn reparse_tag(path: &Path) -> Option<u32> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FileAttributeTagInfo, GetFileInformationByHandleEx, FILE_ATTRIBUTE_TAG_INFO,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    let file = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .ok()?;
    // SAFETY: plain-old-data out parameter, filled by the call below.
    let mut info: FILE_ATTRIBUTE_TAG_INFO = unsafe { std::mem::zeroed() };
    // SAFETY: the handle is owned by `file` and stays open; the size is `info`'s.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileAttributeTagInfo,
            (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    };
    (ok != 0 && info.FileAttributes & 0x400 != 0).then_some(info.ReparseTag)
}
pub(super) fn same_directory(path: &Path, expected: &Identity) -> Result<(), Error> {
    let actual = identity(path)?;
    if !path.is_dir() || actual.device != expected.device || actual.inode != expected.inode {
        return Err(StorageError::UnsafeFile.into());
    }
    Ok(())
}
pub(super) fn open(path: &Path) -> Result<File, Error> {
    let before = identity(path)?;
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
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() || file_identity(&file)? != before {
        return Err(StorageError::UnsafeFile.into());
    }
    Ok(file)
}
pub(super) fn hash(path: &Path, cancel: &CancellationToken) -> Result<Entry, Error> {
    let mut file = open(path)?;
    let before = file_identity(&file)?;
    let mut sha = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    let mut size = 0;
    loop {
        check_cancel(cancel)?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        sha.update(&buffer[..n]);
        size += n as u64;
    }
    if identity(path)? != before || file_identity(&file)? != before || size != before.length {
        let check = rejection::Rejection::new("folder_changed_during_check")
            .entry(rejection::relative(path.parent().unwrap_or(path), path))
            .expected(before.length.to_string())
            .found(size.to_string());
        return Err(rejection::reject(Error::SourceChanged, check));
    }
    Ok(Entry {
        sha256: sha.finalize().into(),
        size,
        identity: before,
    })
}
pub(super) fn scan(root: &Path, cancel: &CancellationToken) -> Result<Index, Error> {
    scan_inner(root, cancel, &mut |_| {})
}
/// The walk behind [`scan`]; `on_file` sees the size of each file it hashes.
fn scan_inner(
    root: &Path,
    cancel: &CancellationToken,
    on_file: &mut dyn FnMut(u64),
) -> Result<Index, Error> {
    fn walk(
        root: &Path,
        path: &Path,
        depth: usize,
        out: &mut Index,
        folded: &mut BTreeMap<String, String>,
        cancel: &CancellationToken,
        on_file: &mut dyn FnMut(u64),
    ) -> Result<(), Error> {
        check_cancel(cancel)?;
        if depth > MAX_DEPTH {
            let check = rejection::Rejection::new("folder_too_deep")
                .entry(rejection::relative(root, path))
                .expected(MAX_DEPTH.to_string())
                .found(depth.to_string());
            return Err(rejection::reject(StorageError::TooLarge.into(), check));
        }
        if out.len() > MAX_FILES {
            return Err(too_large(root, path, out.len()));
        }
        identity(path)?;
        for child in std::fs::read_dir(path)? {
            if out.len() >= MAX_FILES || folded.len() >= MAX_FILES {
                return Err(too_large(root, path, folded.len()));
            }
            let child = child?;
            let path = child.path();
            identity(&path)?;
            // Index names use `/` on every host, so a Windows scan compares
            // with the reference and with a plan written elsewhere.
            let mut parts = Vec::new();
            for part in path
                .strip_prefix(root)
                .map_err(|_| StorageError::UnsafeFile)?
                .components()
            {
                match part {
                    std::path::Component::Normal(part) => {
                        parts.push(part.to_str().ok_or(StorageError::UnsafeFile)?)
                    }
                    _ => return Err(StorageError::UnsafeFile.into()),
                }
            }
            let name = parts.join("/");
            if !scan_name_allowed(&name) {
                return Err(StorageError::UnsafeFile.into());
            }
            if folded
                .insert(name.to_ascii_lowercase(), name.clone())
                .is_some()
            {
                return Err(StorageError::UnsafeFile.into());
            }
            if child.file_type()?.is_dir() {
                walk(root, &path, depth + 1, out, folded, cancel, on_file)?;
            } else {
                let entry = hash(&path, cancel)?;
                let size = entry.size;
                out.insert(name, entry);
                on_file(size);
            }
        }
        Ok(())
    }
    let before = identity(root)?;
    let mut index = Index::new();
    walk(
        root,
        root,
        0,
        &mut index,
        &mut BTreeMap::new(),
        cancel,
        on_file,
    )?;
    if identity(root)? != before {
        let check = rejection::Rejection::new("folder_changed_during_check")
            .entry(".")
            .expected("same folder")
            .found("replaced");
        return Err(rejection::reject(Error::SourceChanged, check));
    }
    Ok(index)
}
/// The bytes and files `scan` would walk, from metadata only. It never opens or
/// hashes a file and never rejects: the walk does the safety checks.
fn sized(root: &Path, cancel: &CancellationToken) -> Result<u64, Error> {
    fn add(
        path: &Path,
        depth: usize,
        files: &mut usize,
        total: &mut u64,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        check_cancel(cancel)?;
        if depth > MAX_DEPTH {
            return Ok(());
        }
        let Ok(children) = std::fs::read_dir(path) else {
            return Ok(());
        };
        for child in children.flatten() {
            if *files >= MAX_FILES {
                return Ok(());
            }
            let Ok(kind) = child.file_type() else {
                continue;
            };
            if kind.is_dir() {
                add(&child.path(), depth + 1, files, total, cancel)?;
            } else if kind.is_file() {
                *files += 1;
                *total = total.saturating_add(child.metadata().map_or(0, |m| m.len()));
            }
        }
        Ok(())
    }
    let (mut files, mut total) = (0, 0);
    add(root, 0, &mut files, &mut total, cancel)?;
    Ok(total)
}
/// [`scan`] with progress: `on_bytes(hashed, total)` once after sizing, then after
/// each hashed file. `total` is the sized total, raised when a file grew after it.
pub(super) fn scan_with_progress(
    root: &Path,
    cancel: &CancellationToken,
    on_bytes: &dyn Fn(u64, u64),
) -> Result<Index, Error> {
    let total = sized(root, cancel)?;
    on_bytes(0, total);
    let mut hashed = 0u64;
    scan_inner(root, cancel, &mut |size| {
        hashed = hashed.saturating_add(size);
        // A file that grew after sizing never shows hashed > total.
        on_bytes(hashed, total.max(hashed));
    })
}

/// `TooLarge` for a folder past the file cap: the walked folder and the count seen.
fn too_large(root: &Path, path: &Path, count: usize) -> Error {
    let check = rejection::Rejection::new("folder_too_large")
        .entry(rejection::relative(root, path))
        .expected(MAX_FILES.to_string())
        .found(count.to_string());
    rejection::reject(StorageError::TooLarge.into(), check)
}

/// The size and digest of `entry` in the C14 `digest()` form.
fn entry_digest(size: u64, sha256: &[u8]) -> String {
    let hex: String = sha256.iter().map(|b| format!("{b:02x}")).collect();
    rejection::digest(size, &hex)
}
pub(super) fn content_digest(index: &Index) -> Result<[u8; 32], Error> {
    digest(
        &index
            .iter()
            .map(|(path, entry)| (path, entry.size, entry.sha256))
            .collect::<Vec<_>>(),
    )
}
pub(super) fn copy(
    source: &Path,
    dest: &Path,
    expected: &Entry,
    cancel: &CancellationToken,
) -> Result<(), Error> {
    let mut input = open(source)?;
    if file_identity(&input)? != expected.identity {
        let check = rejection::Rejection::new("folder_changed_during_check")
            .entry(rejection::relative(
                source.parent().unwrap_or(source),
                source,
            ))
            .expected(entry_digest(expected.size, &expected.sha256))
            .found("replaced");
        return Err(rejection::reject(Error::SourceChanged, check));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut output = OpenOptions::new().write(true).create_new(true).open(dest)?;
    let mut sha = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    let mut size = 0;
    loop {
        check_cancel(cancel)?;
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        output.write_all(&buffer[..n])?;
        sha.update(&buffer[..n]);
        size += n as u64;
    }
    let actual: [u8; 32] = sha.finalize().into();
    if size != expected.size || actual != expected.sha256 || identity(source)? != expected.identity
    {
        let check = rejection::Rejection::new("folder_changed_during_check")
            .entry(rejection::relative(
                source.parent().unwrap_or(source),
                source,
            ))
            .expected(entry_digest(expected.size, &expected.sha256))
            .found(entry_digest(size, &actual));
        return Err(rejection::reject(Error::SourceChanged, check));
    }
    output.set_times(std::fs::FileTimes::new().set_modified(input.metadata()?.modified()?))?;
    output.sync_all()?;
    Ok(())
}
pub(super) fn check_cancel(cancel: &CancellationToken) -> Result<(), Error> {
    if cancel.is_cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn too_large_records_the_walked_folder_and_the_count() {
        let root = Path::new("base");
        let (error, first) = rejection::scope(|| too_large(root, &root.join("sub"), 200_001));
        let first = first.expect("the too-large check is recorded");
        assert_eq!(first.check, "folder_too_large");
        assert_eq!(first.entry.as_deref(), Some("sub"));
        assert_eq!(first.expected.as_deref(), Some("200000"));
        assert_eq!(first.found.as_deref(), Some("200001"));
        assert_eq!(rejection::code(&error), "corrupt_state");
    }
}
