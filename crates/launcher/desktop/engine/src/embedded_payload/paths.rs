//! Refuse redirected cache paths and retain Windows directory identities.
use super::*;
fn plain(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    !metadata.is_symlink()
}
pub(super) fn directory(path: &Path, create: bool) -> io::Result<File> {
    if create {
        match fs::symlink_metadata(path) {
            Ok(_) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => match fs::create_dir(path) {
                Ok(()) => (),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
                Err(error) => return Err(error),
            },
            Err(error) => return Err(error),
        }
    }
    let mut options = File::options();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::*;
        options
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || !plain(&metadata) || !plain(&fs::symlink_metadata(path)?) {
        return Err(invalid());
    }
    Ok(file)
}
pub(super) fn create_tree(path: &Path) -> io::Result<Vec<File>> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(invalid());
    }
    let mut ancestors: Vec<_> = path.ancestors().collect();
    ancestors.reverse();
    ancestors
        .into_iter()
        .map(|path| directory(path, true))
        .collect()
}
pub(super) fn plain_file(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || !plain(&metadata) {
        return Err(invalid());
    }
    Ok(())
}
pub(super) fn open_file(path: &Path) -> io::Result<File> {
    match fs::symlink_metadata(path) {
        Ok(_) => plain_file(path)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        Err(error) => return Err(error),
    }
    let mut options = File::options();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    if !plain(&file.metadata()?) || !file.metadata()?.is_file() {
        return Err(invalid());
    }
    Ok(file)
}
pub(super) fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => plain_file(path)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        Err(error) => return Err(error),
    }
    let mut temporary = tempfile::NamedTempFile::new_in(path.parent().ok_or_else(invalid)?)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    plain_file(path)
}
