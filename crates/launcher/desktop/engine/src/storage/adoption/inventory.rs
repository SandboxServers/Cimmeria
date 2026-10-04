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
    device: u64,
    inode: u64,
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
pub(super) fn identity(path: &Path) -> Result<Identity, Error> {
    let metadata = std::fs::symlink_metadata(path)?;
    identity_of(&metadata)
}
fn identity_of(metadata: &std::fs::Metadata) -> Result<Identity, Error> {
    if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
        return Err(StorageError::UnsafeFile.into());
    }
    #[cfg(unix)]
    {
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
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Err(StorageError::UnsafeFile.into())
    }
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
    let file = options.open(path)?;
    if !file.metadata()?.is_file() || identity_of(&file.metadata()?)? != before {
        return Err(StorageError::UnsafeFile.into());
    }
    Ok(file)
}
pub(super) fn hash(path: &Path, cancel: &CancellationToken) -> Result<Entry, Error> {
    let mut file = open(path)?;
    let before = identity_of(&file.metadata()?)?;
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
    if identity(path)? != before
        || identity_of(&file.metadata()?)? != before
        || size != before.length
    {
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
            let name = path
                .strip_prefix(root)
                .map_err(|_| StorageError::UnsafeFile)?
                .to_str()
                .ok_or(StorageError::UnsafeFile)?
                .to_string();
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
    if identity_of(&input.metadata()?)? != expected.identity {
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
