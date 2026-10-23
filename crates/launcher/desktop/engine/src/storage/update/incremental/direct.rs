//! Seed direct-root updates without traversing launcher work or ownership files.
use super::*;
use std::io;

pub(super) fn seed(
    root: &Path,
    target: &Path,
    writes: &BTreeSet<String>,
    cancel: &CancellationToken,
) -> io::Result<SeedStats> {
    if target.try_exists()? {
        return Err(io::ErrorKind::AlreadyExists.into());
    }
    let mut files = Vec::new();
    walk(root, root, 0, &mut files)?;
    std::fs::create_dir_all(target)?;
    let mut stats = SeedStats::default();
    for relative in files {
        if cancel.is_cancelled() {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let from = root.join(&relative);
        let to = target.join(&relative);
        std::fs::create_dir_all(to.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
        let linked = cfg!(windows)
            && !writes.contains(&key(&relative.to_string_lossy()))
            && std::fs::hard_link(&from, &to).is_ok();
        if linked {
            stats.linked += 1;
        } else {
            stats.bytes_copied += std::fs::copy(from, to)?;
            stats.copied += 1;
        }
    }
    Ok(stats)
}
fn walk(root: &Path, dir: &Path, depth: usize, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if depth > 64 || files.len() > 200_000 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if dir == root
            && (entry
                .file_name()
                .to_string_lossy()
                .starts_with(".cimmeria-")
                || entry.file_name() == "content-ready.json")
        {
            continue;
        }
        let meta = std::fs::symlink_metadata(entry.path())?;
        crate::storage::repair::ordinary(&meta).map_err(|_| io::ErrorKind::InvalidInput)?;
        if meta.is_dir() {
            walk(root, &entry.path(), depth + 1, files)?;
        } else if meta.is_file() {
            files.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| io::ErrorKind::InvalidInput)?
                    .to_path_buf(),
            );
        } else {
            return Err(io::ErrorKind::InvalidInput.into());
        }
    }
    Ok(())
}
