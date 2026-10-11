//! Retain vendor installers as inert data, separately from the playable tree.
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::{UnpackError, UnpackSink};

pub(super) const DIRECTORY: &str = ".cimmeria-prerequisites";

#[derive(PartialEq, Eq)]
enum Entry {
    Directory,
    File(u64, [u8; 32]),
}

/// Publish the extracted directory by rename on the same volume. A retry may
/// retain an identical directory, but must never replace unrelated content.
/// Presence here does not mean any prerequisite has been installed.
pub(super) fn preserve(staging: &Path, dest: &Path, sink: &UnpackSink) -> Result<(), UnpackError> {
    sink.check_cancel()?;
    let source = staging.join("Data").join("Prerequisites");
    match fs::symlink_metadata(&source) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        result => {
            result?;
        }
    }
    // Check the parent too: traversing a junction/symlink before inspecting
    // the leaf would allow an otherwise regular directory outside staging.
    require_directory(&staging.join("Data"))?;
    let expected = inventory(&source, sink)?;
    let target = dest.join(DIRECTORY);
    match fs::symlink_metadata(&target) {
        Ok(_) => {
            if inventory(&target, sink)? != expected {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "retained prerequisite directory differs from archive; refusing replacement",
                )
                .into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            sink.check_cancel()?;
            fs::rename(source, target)?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn ordinary(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    !metadata.file_type().is_symlink() && (metadata.is_file() || metadata.is_dir())
}

fn require_directory(path: &Path) -> Result<(), UnpackError> {
    let metadata = fs::symlink_metadata(path)?;
    if !ordinary(&metadata) || !metadata.is_dir() {
        return Err(UnpackError::UnsafePath(path.display().to_string()));
    }
    Ok(())
}

fn inventory(root: &Path, sink: &UnpackSink) -> Result<BTreeMap<PathBuf, Entry>, UnpackError> {
    require_directory(root)?;
    let mut result = BTreeMap::new();
    visit(root, root, sink, &mut result)?;
    Ok(result)
}

fn visit(
    root: &Path,
    directory: &Path,
    sink: &UnpackSink,
    entries: &mut BTreeMap<PathBuf, Entry>,
) -> Result<(), UnpackError> {
    for item in fs::read_dir(directory)? {
        sink.check_cancel()?;
        let path = item?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if !ordinary(&metadata) {
            return Err(UnpackError::UnsafePath(path.display().to_string()));
        }
        let relative = path
            .strip_prefix(root)
            .expect("walk starts at root")
            .to_owned();
        if metadata.is_dir() {
            entries.insert(relative, Entry::Directory);
            visit(root, &path, sink, entries)?;
        } else {
            let mut file = fs::File::open(&path)?;
            let mut hash = Sha256::new();
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                sink.check_cancel()?;
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
            }
            entries.insert(
                relative,
                Entry::File(metadata.len(), hash.finalize().into()),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unpack::test_fixtures::sink;

    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let staging = temp.path().join("staging");
        let dest = temp.path().join("game");
        fs::create_dir_all(staging.join("Data/Prerequisites/DX9.0c")).unwrap();
        fs::create_dir(&dest).unwrap();
        fs::write(
            staging.join("Data/Prerequisites/DX9.0c/DXSETUP.exe"),
            b"inert fixture",
        )
        .unwrap();
        (temp, staging, dest)
    }

    #[test]
    fn preserves_vendor_tree_without_executing_and_accepts_identical_retry() {
        let (_temp, staging, dest) = fixture();
        let (sink, _rx) = sink();
        preserve(&staging, &dest, &sink).unwrap();
        assert_eq!(
            fs::read(dest.join(DIRECTORY).join("DX9.0c/DXSETUP.exe")).unwrap(),
            b"inert fixture"
        );
        assert!(!staging.join("Data/Prerequisites").exists());
        fs::create_dir_all(staging.join("Data/Prerequisites/DX9.0c")).unwrap();
        fs::write(
            staging.join("Data/Prerequisites/DX9.0c/DXSETUP.exe"),
            b"inert fixture",
        )
        .unwrap();
        preserve(&staging, &dest, &sink).unwrap();
        assert!(staging.join("Data/Prerequisites").exists());
    }

    #[test]
    fn differing_or_extra_files_are_not_overwritten() {
        let (_temp, staging, dest) = fixture();
        let (sink, _rx) = sink();
        fs::create_dir_all(dest.join(DIRECTORY).join("DX9.0c")).unwrap();
        let target = dest.join(DIRECTORY).join("DX9.0c/DXSETUP.exe");
        fs::write(&target, b"foreign").unwrap();
        assert!(preserve(&staging, &dest, &sink).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"foreign");
        fs::write(&target, b"inert fixture").unwrap();
        fs::write(dest.join(DIRECTORY).join("extra"), b"foreign").unwrap();
        assert!(preserve(&staging, &dest, &sink).is_err());
        assert!(staging.join("Data/Prerequisites").exists());
    }

    #[test]
    fn missing_payload_is_optional_and_cancellation_does_not_publish() {
        let (_temp, staging, dest) = fixture();
        let (sink, _rx) = sink();
        sink.cancel.cancel();
        assert!(matches!(
            preserve(&staging, &dest, &sink),
            Err(UnpackError::Cancelled)
        ));
        assert!(!dest.join(DIRECTORY).exists());
        fs::remove_dir_all(staging.join("Data/Prerequisites")).unwrap();
        let (sink, _rx) = crate::unpack::test_fixtures::sink();
        preserve(&staging, &dest, &sink).unwrap();
        assert!(!dest.join(DIRECTORY).exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_source_and_destination_links_without_following_them() {
        use std::os::unix::fs::symlink;
        let (temp, staging, dest) = fixture();
        let (sink, _rx) = sink();
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        symlink(&outside, dest.join(DIRECTORY)).unwrap();
        assert!(preserve(&staging, &dest, &sink).is_err());
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
        fs::remove_file(dest.join(DIRECTORY)).unwrap();
        symlink(&outside, staging.join("Data/Prerequisites/link")).unwrap();
        assert!(preserve(&staging, &dest, &sink).is_err());
        assert!(!dest.join(DIRECTORY).exists());
    }
}
