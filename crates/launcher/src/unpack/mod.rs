//! Unpack a downloaded seed or patch archive into the install directory.
//!
//! Two archive formats are accepted, told apart by their magic bytes rather
//! than the file name (the download lands in a `.tmp-*` file):
//!
//! - **zip** — extracted straight into the destination.
//! - **RAR** — extracted into a staging directory first. The archived 2009
//!   client (archive.org item `StargateWorlds_0.8348.1.4046`) is a RAR
//!   holding the original installer: `SetupQA.exe` plus a MakeCAB cabinet
//!   set (`Data\DATA.INF` and `Data\DATA1.CAB`..`DATA4.CAB`). When the
//!   staging tree holds such a set, [`cab_set`] expands the cabinets into
//!   the destination, which yields the installed layout
//!   (`Common\`, `Resources\`, `Working\Binaries\SGW.exe`) without running
//!   the installer. Otherwise the staged files are moved across as they are.
//!
//! Everything here is blocking file I/O; callers run it on a blocking thread.

mod cab_set;
mod dos_time;
#[cfg(windows)]
mod fdi;
mod rar;
mod zip;

use std::io::Read;
use std::path::{Path, PathBuf};

use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::install::Progress;

/// Name of the staging directory a RAR is extracted into, under the
/// destination. Removed when unpacking finishes, and cleared before a retry.
const STAGING_DIR: &str = ".tmp-unpack";

#[derive(Debug, Error)]
pub enum UnpackError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] ::zip::result::ZipError),
    #[error("RAR error: {0}")]
    Rar(String),
    #[error("Cabinet error: {0}")]
    Cab(String),
    #[error("{0} is neither a zip nor a RAR archive")]
    UnknownFormat(PathBuf),
    #[error("Archive entry {0:?} would land outside the install directory")]
    UnsafePath(String),
    #[error("Patch set error: {0}")]
    Patchset(String),
    #[error("Cancelled")]
    Cancelled,
}

/// Where unpack progress goes, and the cancel switch it honours. Owned
/// (not borrowed) so it can move onto a blocking thread.
#[derive(Clone)]
pub struct UnpackSink {
    pub progress: tokio::sync::mpsc::UnboundedSender<Progress>,
    pub label: String,
    pub cancel: CancellationToken,
}

impl UnpackSink {
    fn check_cancel(&self) -> Result<(), UnpackError> {
        if self.cancel.is_cancelled() {
            Err(UnpackError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn report(&self, step: &str, current: usize, total: usize, path: &Path) {
        let _ = self.progress.send(Progress::Extracting {
            label: format!("{} ({step})", self.label),
            current,
            total,
            filename: path.display().to_string(),
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    Rar,
}

/// Identify an archive from its first bytes.
pub fn detect(path: &Path) -> Result<ArchiveKind, UnpackError> {
    let mut magic = [0u8; 7];
    let mut f = std::fs::File::open(path)?;
    let n = f.read(&mut magic)?;
    let magic = &magic[..n];
    if magic.starts_with(b"PK\x03\x04") || magic.starts_with(b"PK\x05\x06") {
        Ok(ArchiveKind::Zip)
    } else if magic.starts_with(b"Rar!\x1a\x07") {
        // RAR4 is `Rar!\x1a\x07\x00`, RAR5 is `Rar!\x1a\x07\x01\x00`.
        Ok(ArchiveKind::Rar)
    } else {
        Err(UnpackError::UnknownFormat(path.to_path_buf()))
    }
}

/// Unpack `archive` into `dest`, overwriting files that already exist.
pub fn unpack(archive: &Path, dest: &Path, sink: &UnpackSink) -> Result<(), UnpackError> {
    std::fs::create_dir_all(dest)?;
    match detect(archive)? {
        ArchiveKind::Zip => zip::extract(archive, dest, sink),
        ArchiveKind::Rar => {
            let staging = dest.join(STAGING_DIR);
            if staging.exists() {
                std::fs::remove_dir_all(&staging)?;
            }
            rar::extract(archive, &staging, sink)?;
            if let Some(set) = cab_set::find_installer(&staging)? {
                info!(
                    cabinets = set.cabinets.len(),
                    files = set.file_count,
                    dir = %set.dir.display(),
                    "RAR holds an installer cabinet set; expanding it"
                );
                cab_set::expand(&set, dest, sink)?;
            } else {
                move_tree(&staging, &staging, dest)?;
            }
            std::fs::remove_dir_all(&staging)?;
            Ok(())
        }
    }
}

/// Turn an archive entry name into a relative path that stays inside the
/// destination. Accepts `\` and `/` separators; rejects `..`, drive letters
/// and names that reduce to nothing.
pub(crate) fn safe_relative(name: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in name.split(['\\', '/']) {
        match part {
            "" | "." => continue,
            ".." => return None,
            p if p.contains(':') => return None,
            p => out.push(p),
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

/// Move every file under `dir` into the same relative place under `dest`,
/// replacing what is there. `root` is the top of the tree being moved.
fn move_tree(root: &Path, dir: &Path, dest: &Path) -> Result<(), UnpackError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(root).expect("walked from root");
        let target = dest.join(rel);
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&target)?;
            move_tree(root, &path, dest)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Same volume (staging lives under dest), so this is a rename,
            // and rename replaces an existing file.
            std::fs::rename(&path, &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod test_fixtures;

#[cfg(test)]
mod tests {
    use super::test_fixtures::{sink, write_stored_rar4};
    use super::*;

    #[test]
    fn safe_relative_accepts_both_separators() {
        assert_eq!(
            safe_relative("Working\\binaries/SGW.exe").unwrap(),
            PathBuf::from("Working").join("binaries").join("SGW.exe")
        );
        assert_eq!(
            safe_relative("./a//b").unwrap(),
            PathBuf::from("a").join("b")
        );
    }

    #[test]
    fn safe_relative_rejects_escapes() {
        for bad in ["..\\evil.dll", "a/../../b", "C:\\Windows\\x", "", "\\", "."] {
            assert!(safe_relative(bad).is_none(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn detect_tells_zip_rar_and_junk_apart() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("a");
        std::fs::write(&zip, b"PK\x03\x04rest").unwrap();
        let rar = dir.path().join("b");
        std::fs::write(&rar, b"Rar!\x1a\x07\x00rest").unwrap();
        let junk = dir.path().join("c");
        std::fs::write(&junk, b"MZ\x90\x00").unwrap();
        assert_eq!(detect(&zip).unwrap(), ArchiveKind::Zip);
        assert_eq!(detect(&rar).unwrap(), ArchiveKind::Rar);
        assert!(matches!(detect(&junk), Err(UnpackError::UnknownFormat(_))));
    }

    // A RAR without an installer inside is plain content: its files land
    // in dest at their archived paths, and the staging dir is cleaned up.
    #[test]
    fn plain_rar_is_moved_into_dest() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("p.rar");
        write_stored_rar4(
            &archive,
            &[("Working\\binaries\\note.txt", b"hello"), ("top.txt", b"x")],
        );
        let dest = dir.path().join("install");
        std::fs::create_dir_all(dest.join("Working").join("binaries")).unwrap();
        std::fs::write(dest.join("top.txt"), b"old").unwrap();
        let (sink, _rx) = sink();
        unpack(&archive, &dest, &sink).unwrap();
        assert_eq!(
            std::fs::read(dest.join("Working").join("binaries").join("note.txt")).unwrap(),
            b"hello"
        );
        assert_eq!(std::fs::read(dest.join("top.txt")).unwrap(), b"x");
        // The move from staging is a rename, so UnRAR's restored mtime
        // survives into the install.
        assert_eq!(
            std::fs::metadata(dest.join("top.txt"))
                .unwrap()
                .modified()
                .unwrap(),
            super::test_fixtures::fixture_mtime()
        );
        assert!(!dest.join(STAGING_DIR).exists());
    }

    // The archive.org client shape end to end: a RAR holding SetupQA.exe
    // and a spanning MakeCAB set under Data\ unpacks to the installed
    // layout, and neither the installer nor the cabinets land in dest.
    #[cfg(windows)]
    #[test]
    fn installer_rar_expands_to_the_installed_layout() {
        let dir = tempfile::tempdir().unwrap();
        let cab_dir = dir.path().join("cabs");
        std::fs::create_dir_all(&cab_dir).unwrap();
        let sgw = super::test_fixtures::incompressible(7, 150_000);
        let files = vec![
            ("Working\\binaries\\SGW.exe", sgw.clone()),
            ("Common\\res\\a.def", b"<def/>".to_vec()),
        ];
        let cabs = super::test_fixtures::make_cab_set(&cab_dir, &files, 65_536);
        assert!(cabs.len() >= 2, "{cabs:?}");

        let mut entries: Vec<(String, Vec<u8>)> = vec![("SetupQA.exe".into(), b"MZ".to_vec())];
        for name in cabs.iter().map(String::as_str).chain(["DATA.INF"]) {
            entries.push((
                format!("Data\\{name}"),
                std::fs::read(cab_dir.join(name)).unwrap(),
            ));
        }
        let refs: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        let archive = dir.path().join("client.rar");
        write_stored_rar4(&archive, &refs);

        let dest = dir.path().join("install");
        let (sink, _rx) = sink();
        unpack(&archive, &dest, &sink).unwrap();
        assert_eq!(
            std::fs::read(dest.join("Working").join("binaries").join("SGW.exe")).unwrap(),
            sgw
        );
        assert!(dest.join("Common").join("res").join("a.def").is_file());
        // End to end, the installed files carry the cabinet's date, not the
        // install time (UE3's "ini file is outdated" dialog).
        assert_eq!(
            std::fs::metadata(dest.join("Common").join("res").join("a.def"))
                .unwrap()
                .modified()
                .unwrap(),
            super::test_fixtures::fixture_mtime()
        );
        assert!(!dest.join("SetupQA.exe").exists());
        assert!(!dest.join("Data").exists());
        assert!(!dest.join(STAGING_DIR).exists());
        assert_eq!(
            crate::install_layout::sgw_exe(&dest),
            dest.join("Working").join("binaries").join("SGW.exe")
        );
    }

    // Manual check against the real archive.org client RAR (4.1 GB, about
    // 10 GB of scratch space). Run with:
    //   SGW_CLIENT_RAR=<path to .rar> SGW_UNPACK_DEST=<empty dir> \
    //     cargo test -p sgw-launcher real_client_rar -- --ignored --nocapture
    #[test]
    #[ignore = "needs the 4.1 GB client RAR; see the comment"]
    fn real_client_rar() {
        let archive = PathBuf::from(std::env::var("SGW_CLIENT_RAR").unwrap());
        let dest = PathBuf::from(std::env::var("SGW_UNPACK_DEST").unwrap());
        let (sink, mut rx) = sink();
        let started = std::time::Instant::now();
        unpack(&archive, &dest, &sink).unwrap();
        let mut events = 0;
        while rx.try_recv().is_ok() {
            events += 1;
        }
        println!(
            "unpacked in {:?}, {events} progress events",
            started.elapsed()
        );
        assert!(crate::install_layout::sgw_exe(&dest).is_file());
    }

    // A leftover staging dir from a crashed run must not leak stale files
    // into the next install.
    #[test]
    fn stale_staging_is_cleared_before_a_retry() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("p.rar");
        write_stored_rar4(&archive, &[("fresh.txt", b"new")]);
        let dest = dir.path().join("install");
        std::fs::create_dir_all(dest.join(STAGING_DIR)).unwrap();
        std::fs::write(dest.join(STAGING_DIR).join("stale.txt"), b"old").unwrap();
        let (sink, _rx) = sink();
        unpack(&archive, &dest, &sink).unwrap();
        assert!(dest.join("fresh.txt").is_file());
        assert!(!dest.join("stale.txt").exists());
    }
}
