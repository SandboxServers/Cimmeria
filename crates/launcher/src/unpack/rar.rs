//! RAR extraction through the UnRAR library (`unrar` crate).
//!
//! The archived 2009 client is a single-volume RAR 4 archive with no
//! compression and no encryption. Multi-volume sets work too as long as the
//! later volumes sit next to the first one, which is how UnRAR finds them.

use std::path::Path;

use unrar::Archive;

use super::{safe_relative, UnpackError, UnpackSink};

fn rar_err(e: unrar::error::UnrarError) -> UnpackError {
    UnpackError::Rar(e.to_string())
}

/// Extract every file of `archive` into `dest`. Entry names are checked
/// with [`safe_relative`] before anything is written, and each file is
/// extracted to that checked path rather than to the name UnRAR reports.
pub(super) fn extract(archive: &Path, dest: &Path, sink: &UnpackSink) -> Result<(), UnpackError> {
    std::fs::create_dir_all(dest)?;
    let total = Archive::new(archive)
        .open_for_listing()
        .map_err(rar_err)?
        .filter(|e| e.as_ref().is_ok_and(|h| h.is_file()))
        .count();

    let mut open = Archive::new(archive)
        .open_for_processing()
        .map_err(rar_err)?;
    let mut done = 0;
    while let Some(header) = open.read_header().map_err(rar_err)? {
        sink.check_cancel()?;
        let entry = header.entry();
        let name = entry.filename.to_string_lossy().into_owned();
        if entry.is_encrypted() {
            return Err(UnpackError::Rar(format!(
                "{name} is encrypted; password-protected archives are not supported"
            )));
        }
        let rel = safe_relative(&name).ok_or_else(|| UnpackError::UnsafePath(name.clone()))?;
        let out = dest.join(rel);
        if entry.is_directory() {
            std::fs::create_dir_all(&out)?;
            open = header.skip().map_err(rar_err)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        open = header.extract_to(&out).map_err(rar_err)?;
        done += 1;
        sink.report("unpacking RAR", done, total, &out);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{sink, write_stored_rar4};
    use super::*;
    use crate::install::Progress;

    #[test]
    fn extracts_files_at_their_archived_paths() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.rar");
        write_stored_rar4(
            &archive,
            &[
                ("SetupQA.exe", b"MZ-installer"),
                ("Data\\DATA.INF", b"[disk list]\r\n"),
            ],
        );
        let out = dir.path().join("out");
        let (sink, mut rx) = sink();
        extract(&archive, &out, &sink).unwrap();
        assert_eq!(
            std::fs::read(out.join("SetupQA.exe")).unwrap(),
            b"MZ-installer"
        );
        assert_eq!(
            std::fs::read(out.join("Data").join("DATA.INF")).unwrap(),
            b"[disk list]\r\n"
        );
        // One progress event per file, counting up to the listed total.
        let mut last = None;
        while let Ok(Progress::Extracting { current, total, .. }) = rx.try_recv() {
            last = Some((current, total));
        }
        assert_eq!(last, Some((2, 2)));
    }

    // Bug shape: an entry named `..\x` must not be written above `dest`.
    #[test]
    fn rejects_entries_that_escape_the_destination() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("evil.rar");
        write_stored_rar4(&archive, &[("..\\escaped.txt", b"x")]);
        let out = dir.path().join("out");
        let (sink, _rx) = sink();
        let err = extract(&archive, &out, &sink).unwrap_err();
        assert!(matches!(err, UnpackError::UnsafePath(_)), "{err:?}");
        assert!(!dir.path().join("escaped.txt").exists());
    }

    #[test]
    fn honours_cancel() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.rar");
        write_stored_rar4(&archive, &[("a.txt", b"a")]);
        let (sink, _rx) = sink();
        sink.cancel.cancel();
        let err = extract(&archive, &dir.path().join("out"), &sink).unwrap_err();
        assert!(matches!(err, UnpackError::Cancelled));
    }

    #[test]
    fn corrupt_archive_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bad.rar");
        std::fs::write(&archive, b"Rar!\x1a\x07\x00garbage-after-the-marker").unwrap();
        let (sink, _rx) = sink();
        assert!(extract(&archive, &dir.path().join("out"), &sink).is_err());
    }
}
