//! Seeding a stage tree from a source tree, with hard links where it is safe.
//!
//! [`seed_stage`] builds a new directory from an existing tree. A file outside
//! the write set is hard-linked, so the stage costs no disk space for it; a
//! file that cannot be linked (on another volume, say) is copied. A file in the
//! write set is always copied, so writing to the stage never changes the
//! source.
//!
//! The walk never follows links. It refuses a symbolic link or reparse point
//! anywhere in the source before it writes anything.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Files between two `cancel` checks.
const CANCEL_EVERY: usize = 64;
/// `FILE_ATTRIBUTE_REPARSE_POINT`: set on junctions, symbolic links and other reparse points.
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// What [`seed_stage`] did: how many files were linked, how many were copied,
/// and how many bytes the copies wrote.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SeedStats {
    /// Files that share their content with the source through a hard link.
    pub linked: u64,
    /// Files copied: the write set, and any link that failed.
    pub copied: u64,
    /// Bytes written by the copies.
    pub bytes_copied: u64,
}

/// Builds the tree `dst` from `src`.
///
/// `dst` must not exist. Every directory under `src` is created under `dst`.
/// A file's key is its lower-cased, `/`-separated relative path. With `link`
/// set, a file whose key is not in `write_set` is hard-linked, and copied if
/// the link fails; every other file is copied. `cancel` is checked every 64
/// files and stops the seed with [`io::ErrorKind::Interrupted`]. A symbolic
/// link or reparse point anywhere in `src` is refused with
/// [`io::ErrorKind::InvalidInput`] before anything is written. A cancelled or
/// failed seed leaves a partial `dst` for the caller to remove.
pub fn seed_stage(
    src: &Path,
    dst: &Path,
    write_set: &BTreeSet<String>,
    link: bool,
    cancel: &dyn Fn() -> bool,
) -> io::Result<SeedStats> {
    if fs::symlink_metadata(dst).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "stage directory already exists",
        ));
    }
    let mut plan = Plan::default();
    walk(src, Path::new(""), &mut plan)?;

    fs::create_dir_all(dst)?;
    for dir in &plan.dirs {
        fs::create_dir_all(dst.join(dir))?;
    }

    let mut stats = SeedStats::default();
    for (index, rel) in plan.files.iter().enumerate() {
        if index % CANCEL_EVERY == 0 && cancel() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "stage seed cancelled",
            ));
        }
        let from = src.join(rel);
        let to = dst.join(rel);
        let linked = link && !write_set.contains(&key(rel)) && fs::hard_link(&from, &to).is_ok();
        if linked {
            stats.linked += 1;
        } else {
            stats.bytes_copied += fs::copy(&from, &to)?;
            stats.copied += 1;
        }
    }
    Ok(stats)
}

/// What the walk found, relative to the source root.
#[derive(Default)]
struct Plan {
    /// Directories, parents before their children.
    dirs: Vec<PathBuf>,
    /// Regular files.
    files: Vec<PathBuf>,
}

/// Lists everything under `src/rel` without following links. Refuses a link.
fn walk(src: &Path, rel: &Path, plan: &mut Plan) -> io::Result<()> {
    for entry in fs::read_dir(src.join(rel))? {
        let entry = entry?;
        let child = rel.join(entry.file_name());
        let meta = fs::symlink_metadata(src.join(&child))?;
        if is_link(&meta) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seed source holds a symbolic link or reparse point",
            ));
        }
        if meta.is_dir() {
            plan.dirs.push(child.clone());
            walk(src, &child, plan)?;
        } else {
            plan.files.push(child);
        }
    }
    Ok(())
}

/// True for a symbolic link, and on Windows for any reparse point.
fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    };
    #[cfg(not(windows))]
    let reparse = false;
    meta.file_type().is_symlink() || reparse
}

/// The write-set key for a relative path: its lower-cased components joined with `/`.
fn key(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source tree with `game/data/a.dat` ("alpha") and `game/b.dat` ("beta").
    fn source() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("game/data")).unwrap();
        fs::write(dir.path().join("game/data/a.dat"), b"alpha").unwrap();
        fs::write(dir.path().join("game/b.dat"), b"beta").unwrap();
        dir
    }

    fn never() -> bool {
        false
    }

    #[test]
    fn write_set_files_are_independent_copies() {
        let tmp = source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("stage");
        let write_set: BTreeSet<String> = ["game/data/a.dat".to_string()].into();

        let stats = seed_stage(tmp.path(), &dst, &write_set, true, &never).unwrap();

        // a.dat is in the write set, so it is copied; b.dat is linked.
        assert_eq!(stats.copied, 1);
        fs::write(dst.join("game/data/a.dat"), b"changed").unwrap();
        assert_eq!(
            fs::read(tmp.path().join("game/data/a.dat")).unwrap(),
            b"alpha"
        );
    }

    #[cfg(windows)]
    #[test]
    fn linked_files_share_content() {
        let tmp = source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("stage");

        let stats = seed_stage(tmp.path(), &dst, &BTreeSet::new(), true, &never).unwrap();

        assert_eq!(stats.linked, 2);
        assert_eq!(stats.copied, 0);
        assert_eq!(stats.bytes_copied, 0);
        assert_eq!(fs::read(dst.join("game/b.dat")).unwrap(), b"beta");
    }

    #[test]
    fn link_false_copies_everything() {
        let tmp = source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("stage");

        let stats = seed_stage(tmp.path(), &dst, &BTreeSet::new(), false, &never).unwrap();

        assert_eq!(stats.linked, 0);
        assert_eq!(stats.copied, 2);
        assert_eq!(stats.bytes_copied, 9);
        assert_eq!(fs::read(dst.join("game/data/a.dat")).unwrap(), b"alpha");
    }

    #[test]
    fn a_symlink_in_src_is_refused() {
        let tmp = source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("stage");
        let target = tmp.path().join("game/data");
        let alias = tmp.path().join("game/alias");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&target, &alias);
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_dir(&target, &alias);
        if made.is_err() {
            // Windows needs Developer Mode or elevation for a symbolic link; without it, skip.
            eprintln!("skipped: this session cannot create a symbolic link");
            return;
        }

        let err = seed_stage(tmp.path(), &dst, &BTreeSet::new(), true, &never).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(!dst.exists());
    }

    #[test]
    fn cancel_interrupts() {
        let tmp = source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("stage");

        let err = seed_stage(tmp.path(), &dst, &BTreeSet::new(), true, &|| true).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
    }

    #[test]
    fn an_existing_dst_is_refused() {
        let tmp = source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("stage");
        fs::create_dir(&dst).unwrap();

        let err = seed_stage(tmp.path(), &dst, &BTreeSet::new(), true, &never).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    }
}
