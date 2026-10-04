//! Bounded single-bundle extraction. Archive names never select the installed target.
use super::Error;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

const MAX_EXPANDED: u64 = 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;

pub(super) fn plain(path: &Path) -> Result<(), Error> {
    if !path.is_absolute() {
        return Err(Error::Target);
    }
    for ancestor in path.ancestors() {
        let meta = fs::symlink_metadata(ancestor).map_err(|_| Error::Target)?;
        if meta.file_type().is_symlink() {
            return Err(Error::Target);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err(Error::Target);
            }
        }
    }
    Ok(())
}
pub(super) fn identity(bundle: &Path) -> Result<(String, String, PathBuf), Error> {
    plain(bundle)?;
    let info = bundle.join("Contents/Info.plist");
    plain(&info)?;
    if fs::metadata(&info).map_err(|_| Error::Package)?.len() > 64 * 1024 {
        return Err(Error::Package);
    }
    let value = plist::Value::from_file(info).map_err(|_| Error::Package)?;
    let dict = value.as_dictionary().ok_or(Error::Package)?;
    let get = |key| {
        dict.get(key)
            .and_then(plist::Value::as_string)
            .ok_or(Error::Package)
    };
    let executable = get("CFBundleExecutable")?;
    if executable.is_empty()
        || Path::new(executable).components().count() != 1
        || !matches!(
            Path::new(executable).components().next(),
            Some(Component::Normal(_))
        )
    {
        return Err(Error::Package);
    }
    let exe = PathBuf::from("Contents/MacOS").join(executable);
    plain(&bundle.join(&exe))?;
    if !bundle.join(&exe).is_file() {
        return Err(Error::Package);
    }
    Ok((
        get("CFBundleIdentifier")?.into(),
        get("CFBundleShortVersionString")?.into(),
        exe,
    ))
}

pub(super) fn extract(bytes: &[u8], stage: &Path, owner: uuid::Uuid) -> Result<PathBuf, Error> {
    // Two passes: no entry is written until the entire namespace is safe.
    let mut names = BTreeSet::new();
    let mut total = 0u64;
    let mut root = None;
    let mut links = Vec::new();
    let mut archive = tar::Archive::new(GzDecoder::new(Cursor::new(bytes)));
    for entry in archive.entries().map_err(|_| Error::Package)? {
        let entry = entry.map_err(|_| Error::Package)?;
        let path = entry.path().map_err(|_| Error::Package)?.into_owned();
        if path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(Error::Package);
        }
        let first = path
            .components()
            .next()
            .ok_or(Error::Package)?
            .as_os_str()
            .to_str()
            .ok_or(Error::Package)?;
        if !first.ends_with(".app") || first.len() < 5 {
            return Err(Error::Package);
        }
        if root.as_deref().is_some_and(|r| r != first) {
            return Err(Error::Package);
        }
        root = Some(first.to_owned());
        let key = path.to_str().ok_or(Error::Package)?.to_lowercase();
        if !names.insert(key) || names.len() > MAX_ENTRIES {
            return Err(Error::Package);
        }
        total = total
            .checked_add(entry.size())
            .filter(|n| *n <= MAX_EXPANDED)
            .ok_or(Error::Size)?;
        let kind = entry.header().entry_type();
        if kind.is_symlink() {
            let link = entry
                .link_name()
                .map_err(|_| Error::Package)?
                .ok_or(Error::Package)?
                .into_owned();
            if link.is_absolute() {
                return Err(Error::Package);
            }
            let mut depth = path.components().count() - 1;
            for c in link.components() {
                match c {
                    Component::Normal(_) => depth += 1,
                    Component::CurDir => {}
                    Component::ParentDir if depth > 1 => depth -= 1,
                    _ => return Err(Error::Package),
                }
            }
            links.push(path);
        } else if !kind.is_file() && !kind.is_dir() {
            return Err(Error::Package);
        }
    }
    // No member may descend through a link, even if its target is inside the bundle.
    for link in &links {
        let prefix = format!("{}/", link.to_str().ok_or(Error::Package)?.to_lowercase());
        if names.iter().any(|name| name.starts_with(&prefix)) {
            return Err(Error::Package);
        }
    }
    // Publish an already marked directory atomically; a colliding directory is
    // never adopted merely because it occupies the expected transaction path.
    let temporary = tempfile::Builder::new()
        .prefix(".cimmeria-stage-")
        .tempdir_in(stage.parent().ok_or(Error::Target)?)
        .map_err(|_| Error::Replace)?;
    fs::write(temporary.path().join(".cimmeria-owner"), owner.to_string())
        .map_err(|_| Error::Replace)?;
    sync_tree(temporary.path())?;
    super::apply::process::rename_new(temporary.path(), stage)?;
    let mut archive = tar::Archive::new(GzDecoder::new(Cursor::new(bytes)));
    archive.set_preserve_permissions(false);
    for entry in archive.entries().map_err(|_| Error::Package)? {
        let mut entry = entry.map_err(|_| Error::Package)?;
        if !entry.unpack_in(stage).map_err(|_| Error::Package)? {
            return Err(Error::Package);
        }
    }
    let bundle = stage.join(root.ok_or(Error::Package)?);
    // Resolve links after extraction; dangling/cyclic/outside links are forbidden.
    for link in links {
        let target = stage
            .join(link)
            .canonicalize()
            .map_err(|_| Error::Package)?;
        if !target.starts_with(&bundle) {
            return Err(Error::Package);
        }
    }
    sync_tree(stage)?;
    Ok(bundle)
}

pub(super) fn fingerprint(path: &Path) -> Result<String, Error> {
    fn visit(
        path: &Path,
        digest: &mut Sha256,
        count: &mut usize,
        total: &mut u64,
    ) -> Result<(), Error> {
        *count += 1;
        if *count > MAX_ENTRIES {
            return Err(Error::Size);
        }
        let meta = fs::symlink_metadata(path).map_err(|_| Error::Target)?;
        if meta.file_type().is_symlink() {
            digest.update(b"link");
            let target = fs::read_link(path).map_err(|_| Error::Target)?;
            let bytes = target.as_os_str().as_encoded_bytes();
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        } else if meta.is_dir() {
            digest.update(b"dir");
            let mut entries = fs::read_dir(path)
                .map_err(|_| Error::Target)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| Error::Target)?;
            entries.sort_by_key(|e| e.file_name());
            digest.update((entries.len() as u64).to_le_bytes());
            for entry in entries {
                let name = entry.file_name();
                digest.update((name.as_encoded_bytes().len() as u64).to_le_bytes());
                digest.update(name.as_encoded_bytes());
                visit(&entry.path(), digest, count, total)?;
            }
        } else if meta.is_file() {
            *total = total
                .checked_add(meta.len())
                .filter(|n| *n <= MAX_EXPANDED)
                .ok_or(Error::Size)?;
            digest.update(b"file");
            digest.update(meta.len().to_le_bytes());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                digest.update((meta.permissions().mode() & 0o777).to_le_bytes());
            }
            let mut file = fs::File::open(path).map_err(|_| Error::Target)?;
            let mut buf = [0; 65536];
            let mut remaining = meta.len();
            while remaining > 0 {
                let limit =
                    usize::try_from(remaining.min(buf.len() as u64)).map_err(|_| Error::Size)?;
                let n = file.read(&mut buf[..limit]).map_err(|_| Error::Target)?;
                if n == 0 {
                    return Err(Error::Target);
                }
                remaining -= n as u64;
                digest.update(&buf[..n]);
            }
            if file.read(&mut buf[..1]).map_err(|_| Error::Target)? != 0 {
                return Err(Error::Target);
            }
        } else {
            return Err(Error::Target);
        }
        Ok(())
    }
    let mut hash = Sha256::new();
    visit(path, &mut hash, &mut 0, &mut 0)?;
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
pub(super) fn sync_tree(path: &Path) -> Result<(), Error> {
    let meta = fs::symlink_metadata(path).map_err(|_| Error::Replace)?;
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|_| Error::Replace)? {
            sync_tree(&entry.map_err(|_| Error::Replace)?.path())?;
        }
    }
    #[cfg(unix)]
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| Error::Replace)?;
    Ok(())
}

pub(super) fn owned_stage(stage: &Path, owner: uuid::Uuid) -> Result<(), Error> {
    plain(stage)?;
    let marker = stage.join(".cimmeria-owner");
    plain(&marker)?;
    if fs::metadata(&marker)
        .map_err(|_| Error::Reconciliation)?
        .len()
        != 36
    {
        return Err(Error::Reconciliation);
    }
    let bytes = fs::read(marker).map_err(|_| Error::Reconciliation)?;
    if bytes != owner.to_string().as_bytes() {
        return Err(Error::Reconciliation);
    }
    Ok(())
}
