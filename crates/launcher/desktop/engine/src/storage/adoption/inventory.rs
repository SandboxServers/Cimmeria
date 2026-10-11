//! Bounded deterministic filesystem evidence. Source file handles never write.
use super::*;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};

const MAX_FILES: usize = 200_000;
const MAX_DEPTH: usize = 64;
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
        return Err(Error::SourceChanged);
    }
    Ok(Entry {
        sha256: sha.finalize().into(),
        size,
        identity: before,
    })
}
pub(super) fn scan(root: &Path, cancel: &CancellationToken) -> Result<Index, Error> {
    fn walk(
        root: &Path,
        path: &Path,
        depth: usize,
        out: &mut Index,
        folded: &mut BTreeMap<String, String>,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        check_cancel(cancel)?;
        if depth > MAX_DEPTH || out.len() > MAX_FILES {
            return Err(StorageError::TooLarge.into());
        }
        identity(path)?;
        for child in std::fs::read_dir(path)? {
            if out.len() >= MAX_FILES || folded.len() >= MAX_FILES {
                return Err(StorageError::TooLarge.into());
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
            if !name.is_ascii() || name.len() > 4096 || name.contains(':') || name.contains('\\') {
                return Err(StorageError::UnsafeFile.into());
            }
            if folded
                .insert(name.to_ascii_lowercase(), name.clone())
                .is_some()
            {
                return Err(StorageError::UnsafeFile.into());
            }
            if child.file_type()?.is_dir() {
                walk(root, &path, depth + 1, out, folded, cancel)?;
            } else {
                out.insert(name, hash(&path, cancel)?);
            }
        }
        Ok(())
    }
    let before = identity(root)?;
    let mut index = Index::new();
    walk(root, root, 0, &mut index, &mut BTreeMap::new(), cancel)?;
    if identity(root)? != before {
        return Err(Error::SourceChanged);
    }
    Ok(index)
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
        return Err(Error::SourceChanged);
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
    if size != expected.size
        || <[u8; 32]>::from(sha.finalize()) != expected.sha256
        || identity(source)? != expected.identity
    {
        return Err(Error::SourceChanged);
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
