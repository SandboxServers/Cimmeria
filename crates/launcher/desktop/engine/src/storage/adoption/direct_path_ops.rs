//! Mutations relative to retained directory handles; never translate a Unix fd to a path.
use super::*;
use std::io::{Read, Write};
#[cfg(unix)]
use std::{
    ffi::{CString, OsStr, OsString},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::{OsStrExt, OsStringExt},
    },
};

impl Parents {
    fn has_parent(&self, path: &Path) -> Result<bool, Error> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| StorageError::UnsafeFile)?;
        if relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(StorageError::UnsafeFile.into());
        }
        Ok(self
            .directories
            .contains_key(relative.parent().unwrap_or(Path::new(""))))
    }
    #[cfg(unix)]
    fn entry(&self, path: &Path) -> Result<(i32, CString), Error> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| StorageError::UnsafeFile)?;
        if relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(StorageError::UnsafeFile.into());
        }
        let parent = relative.parent().unwrap_or(Path::new(""));
        let fd = self
            .directories
            .get(parent)
            .ok_or(StorageError::UnsafeFile)?
            .as_raw_fd();
        let name = relative.file_name().ok_or(StorageError::UnsafeFile)?;
        Ok((fd, c_name(name)?))
    }
    pub(in super::super) fn open_child_directory(
        &self,
        parent: &Path,
        name: &std::ffi::OsStr,
        create: bool,
    ) -> Result<Option<File>, Error> {
        #[cfg(unix)]
        {
            let fd = self
                .directories
                .get(parent)
                .ok_or(StorageError::UnsafeFile)?
                .as_raw_fd();
            let name = c_name(name)?;
            match open_at(fd, &name, libc::O_RDONLY | libc::O_DIRECTORY) {
                Ok(file) => Ok(Some(file)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound && create => {
                    // SAFETY: retained parent fd and a single NUL-terminated component.
                    let result = unsafe { libc::mkdirat(fd, name.as_ptr(), 0o700) };
                    if result != 0
                        && std::io::Error::last_os_error().kind()
                            != std::io::ErrorKind::AlreadyExists
                    {
                        return Err(std::io::Error::last_os_error().into());
                    }
                    Ok(Some(open_at(
                        fd,
                        &name,
                        libc::O_RDONLY | libc::O_DIRECTORY,
                    )?))
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error.into()),
            }
        }
        #[cfg(not(unix))]
        {
            let path = self.child(parent, name)?;
            if !path.exists() && create {
                std::fs::create_dir(&path)?;
            }
            match std::fs::symlink_metadata(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error.into()),
                Ok(_) => Ok(Some(open_directory(&path)?)),
            }
        }
    }
    pub(crate) fn is_dir(&self, path: &Path) -> Result<bool, Error> {
        if !self.has_parent(path)? {
            return Ok(false);
        }
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| StorageError::UnsafeFile)?;
        Ok(self
            .open_child_directory(
                relative.parent().unwrap_or(Path::new("")),
                relative.file_name().ok_or(StorageError::UnsafeFile)?,
                false,
            )?
            .is_some())
    }
    pub(crate) fn open(&self, path: &Path, write: bool, create: bool) -> Result<File, Error> {
        #[cfg(unix)]
        let file = {
            let (fd, name) = self.entry(path)?;
            let mode = if write { libc::O_RDWR } else { libc::O_RDONLY };
            open_at(
                fd,
                &name,
                mode | if create {
                    libc::O_CREAT | libc::O_EXCL
                } else {
                    0
                },
            )?
        };
        #[cfg(not(unix))]
        let file = {
            let mut options = OpenOptions::new();
            options.read(true).write(write).create_new(create);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                use windows_sys::Win32::Storage::FileSystem::*;
                options
                    .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                    .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
            }
            options.open(self.path(path)?)?
        };
        validate_file(&file)?;
        Ok(file)
    }
    pub(crate) fn read(&self, path: &Path) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        self.open(path, false, false)?
            .take(32 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 32 * 1024 * 1024 {
            return Err(StorageError::TooLarge.into());
        }
        Ok(bytes)
    }
    pub(crate) fn present(&self, path: &Path) -> Result<bool, Error> {
        #[cfg(unix)]
        {
            let (fd, name) = self.entry(path)?;
            match open_at(fd, &name, libc::O_RDONLY) {
                Ok(file) => {
                    validate_file(&file)?;
                    Ok(true)
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error.into()),
            }
        }
        #[cfg(not(unix))]
        super::super::in_place::present(&self.path(path)?)
    }

    pub(crate) fn hash(&self, path: &Path) -> Result<super::super::in_place::Hashed, Error> {
        hash_file(&mut self.open(path, false, false)?)
    }
    pub(crate) fn holds(
        &self,
        path: &Path,
        expected: &super::super::in_place::Hashed,
    ) -> Result<bool, Error> {
        if !self.present(path)? {
            return Ok(false);
        }
        let actual = self.hash(path)?;
        Ok(actual.size == expected.size && actual.sha256 == expected.sha256)
    }
    pub(crate) fn rename(&self, from: &Path, to: &Path, exclusive: bool) -> Result<(), Error> {
        #[cfg(unix)]
        {
            let (from_fd, from_name) = self.entry(from)?;
            let (to_fd, to_name) = self.entry(to)?;
            // SAFETY: both retained directory fds and single-component names stay live.
            #[cfg(target_os = "macos")]
            let result = unsafe {
                libc::renameatx_np(
                    from_fd,
                    from_name.as_ptr(),
                    to_fd,
                    to_name.as_ptr(),
                    if exclusive { libc::RENAME_EXCL } else { 0 },
                )
            };
            #[cfg(not(target_os = "macos"))]
            let result = unsafe {
                libc::renameat2(
                    from_fd,
                    from_name.as_ptr(),
                    to_fd,
                    to_name.as_ptr(),
                    if exclusive { libc::RENAME_NOREPLACE } else { 0 },
                )
            };
            if result != 0 {
                let error = std::io::Error::last_os_error();
                if exclusive && error.kind() == std::io::ErrorKind::AlreadyExists {
                    return Err(Error::SourceChanged);
                }
                return Err(error.into());
            }
            self.sync_parent(from)?;
            self.sync_parent(to)?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let (from, to) = (self.path(from)?, self.path(to)?);
            if exclusive {
                promote_new(&from, &to).map_err(|error| {
                    if error.kind() == std::io::ErrorKind::AlreadyExists {
                        Error::SourceChanged
                    } else {
                        error.into()
                    }
                })?;
            } else {
                std::fs::rename(from, to)?;
            }
            Ok(())
        }
    }
    #[cfg(unix)]
    fn sync_parent(&self, path: &Path) -> Result<(), Error> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| StorageError::UnsafeFile)?;
        self.directories
            .get(relative.parent().unwrap_or(Path::new("")))
            .ok_or(StorageError::UnsafeFile)?
            .sync_all()?;
        Ok(())
    }
    pub(crate) fn remove(&self, path: &Path, directory: bool) -> Result<(), Error> {
        if !self.has_parent(path)? {
            return Ok(());
        }
        #[cfg(unix)]
        {
            let (fd, name) = self.entry(path)?;
            unlink_at(fd, &name, directory)?;
        }
        #[cfg(not(unix))]
        {
            let path = self.path(path)?;
            let result = if directory {
                std::fs::remove_dir(path)
            } else {
                std::fs::remove_file(path)
            };
            match result {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
    pub(in super::super) fn copy_from(
        &self,
        from: &Path,
        to: &Path,
        expected: &inventory::Entry,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        let mut input = inventory::open(from)?;
        self.copy_open(
            &mut input,
            to,
            &super::super::in_place::Hashed {
                size: expected.size,
                sha256: expected.sha256,
            },
            cancel,
        )
    }
    fn copy_open(
        &self,
        input: &mut File,
        to: &Path,
        expected: &super::super::in_place::Hashed,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        let before = inventory::file_identity(input)?;
        let mut output = self.open(to, true, true)?;
        let mut hash = Sha256::new();
        let mut size = 0;
        let mut buffer = [0; 128 * 1024];
        loop {
            inventory::check_cancel(cancel)?;
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            output.write_all(&buffer[..count])?;
            hash.update(&buffer[..count]);
            size += count as u64;
        }
        if size != expected.size
            || <[u8; 32]>::from(hash.finalize()) != expected.sha256
            || inventory::file_identity(input)? != before
        {
            return Err(Error::SourceChanged);
        }
        output.set_times(std::fs::FileTimes::new().set_modified(input.metadata()?.modified()?))?;
        output.sync_all()?;
        Ok(())
    }
    pub(crate) fn copy_verified(
        &self,
        from: &Path,
        to: &Path,
        expected: &super::super::in_place::Hashed,
    ) -> Result<(), Error> {
        self.copy_open(
            &mut self.open(from, false, false)?,
            to,
            expected,
            &CancellationToken::new(),
        )
    }
    pub(crate) fn copy_atomic(&self, from: &Path, to: &Path) -> Result<(), Error> {
        let expected = self.hash(from)?;
        let mut name = to.as_os_str().to_owned();
        name.push(".adopt-partial");
        let partial = PathBuf::from(name);
        self.remove(&partial, false)?;
        self.copy_verified(from, &partial, &expected)?;
        self.rename(&partial, to, false)
    }
    pub(crate) fn write(&self, to: &Path, bytes: &[u8]) -> Result<(), Error> {
        let mut name = to.as_os_str().to_owned();
        name.push(".adopt-partial");
        let partial = PathBuf::from(name);
        self.remove(&partial, false)?;
        let mut file = self.open(&partial, true, true)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        self.rename(&partial, to, false)
    }
    pub(crate) fn names(&self, directory: &Path) -> Result<Vec<std::ffi::OsString>, Error> {
        #[cfg(unix)]
        {
            let (fd, name) = self.entry(&directory.join(".enumerate"))?;
            let _ = name;
            names_fd(fd)
        }
        #[cfg(not(unix))]
        {
            std::fs::read_dir(directory)?
                .map(|row| Ok(row?.file_name()))
                .collect()
        }
    }
    pub(crate) fn remove_tree(&self, path: &Path) -> Result<(), Error> {
        if !self.has_parent(path)? {
            return Ok(());
        }
        #[cfg(unix)]
        {
            let (fd, name) = self.entry(path)?;
            remove_tree_at(fd, &name)?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            match std::fs::remove_dir_all(self.path(path)?) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
    }
}
fn validate_file(file: &File) -> Result<(), Error> {
    if !file.metadata()?.is_file() {
        return Err(StorageError::UnsafeFile.into());
    }
    inventory::file_identity(file)?;
    Ok(())
}
fn hash_file(file: &mut File) -> Result<super::super::in_place::Hashed, Error> {
    let before = inventory::file_identity(file)?;
    let mut hash = Sha256::new();
    let mut size = 0;
    let mut buffer = [0; 128 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
        size += count as u64;
    }
    if inventory::file_identity(file)? != before || file.metadata()?.len() != size {
        return Err(Error::SourceChanged);
    }
    Ok(super::super::in_place::Hashed {
        size,
        sha256: hash.finalize().into(),
    })
}
#[cfg(unix)]
fn c_name(name: &OsStr) -> Result<CString, Error> {
    CString::new(name.as_bytes()).map_err(|_| StorageError::UnsafeFile.into())
}
#[cfg(unix)]
fn open_at(fd: i32, name: &CString, flags: i32) -> std::io::Result<File> {
    // SAFETY: live fd and NUL-terminated name; mode is used only with O_CREAT.
    let raw = unsafe {
        libc::openat(
            fd,
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0o600,
        )
    };
    if raw < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: successful openat returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(raw) })
}
#[cfg(unix)]
fn unlink_at(fd: i32, name: &CString, directory: bool) -> std::io::Result<()> {
    // SAFETY: retained fd and single-component name; no link is followed.
    if unsafe {
        libc::unlinkat(
            fd,
            name.as_ptr(),
            if directory { libc::AT_REMOVEDIR } else { 0 },
        )
    } != 0
    {
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error);
        }
    }
    Ok(())
}
#[cfg(unix)]
fn names_fd(fd: i32) -> Result<Vec<OsString>, Error> {
    let dot = CString::new(".").unwrap();
    let copy = open_at(fd, &dot, libc::O_RDONLY | libc::O_DIRECTORY)?;
    use std::os::fd::IntoRawFd;
    let raw = copy.into_raw_fd();
    // SAFETY: descriptor ownership transfers to the DIR on success.
    let stream = unsafe { libc::fdopendir(raw) };
    if stream.is_null() {
        // SAFETY: fdopendir failed, leaving ownership here.
        unsafe { libc::close(raw) };
        return Err(std::io::Error::last_os_error().into());
    }
    struct Stream(*mut libc::DIR);
    impl Drop for Stream {
        fn drop(&mut self) {
            unsafe {
                libc::closedir(self.0);
            }
        }
    }
    let stream = Stream(stream);
    let mut names = Vec::new();
    loop {
        // SAFETY: stream stays live; copy the name before the next readdir.
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            break;
        }
        let bytes = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes != b"." && bytes != b".." {
            names.push(OsString::from_vec(bytes.to_vec()));
        }
    }
    Ok(names)
}
#[cfg(unix)]
fn remove_tree_at(fd: i32, name: &CString) -> Result<(), Error> {
    let directory = match open_at(fd, name, libc::O_RDONLY | libc::O_DIRECTORY) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for child in names_fd(directory.as_raw_fd())? {
        let child = c_name(&child)?;
        match open_at(
            directory.as_raw_fd(),
            &child,
            libc::O_RDONLY | libc::O_DIRECTORY,
        ) {
            Ok(_) => remove_tree_at(directory.as_raw_fd(), &child)?,
            Err(error) if error.raw_os_error() == Some(libc::ENOTDIR) => {
                unlink_at(directory.as_raw_fd(), &child, false)?
            }
            Err(error) => return Err(error.into()),
        }
    }
    unlink_at(fd, name, true)?;
    Ok(())
}
