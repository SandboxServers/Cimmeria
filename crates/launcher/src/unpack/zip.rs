//! Zip extraction (seed and patch zips published by operators).

use std::path::Path;

use super::{dos_time, UnpackError, UnpackSink};

fn patchset_err(e: cimmeria_patchset::PatchsetError) -> UnpackError {
    UnpackError::Patchset(e.to_string())
}

/// Extract every entry of `zip_path` into `dest`, overwriting existing
/// files. Entries whose names would escape `dest` are skipped by
/// `enclosed_name`.
///
/// A zip carrying a `cimmeria-patch.json` recipe is a patch set: it is
/// applied with `cimmeria-patchset`, which rebuilds files from the stock
/// ones in `dest` and then writes the overlay entries.
pub(super) fn extract(zip_path: &Path, dest: &Path, sink: &UnpackSink) -> Result<(), UnpackError> {
    if cimmeria_patchset::apply::has_recipe(zip_path).map_err(patchset_err)? {
        sink.check_cancel()?;
        let mut n = 0;
        let report = cimmeria_patchset::apply(zip_path, dest, &mut |name| {
            n += 1;
            sink.report("patching", n, n, &dest.join(name));
        })
        .map_err(patchset_err)?;
        tracing::info!(
            rebuilt = report.rebuilt.len(),
            already_current = report.already_current.len(),
            overlay_files = report.overlay_files,
            "applied patch set"
        );
        return Ok(());
    }
    let file = std::fs::File::open(zip_path)?;
    let mut archive = ::zip::ZipArchive::new(file)?;
    let total = archive.len();
    for i in 0..total {
        sink.check_cancel()?;
        let mut entry = archive.by_index(i)?;
        // SECURITY: `enclosed_name()` is the zip-slip gate. It rejects
        // entries whose normalised path would escape the archive root —
        // absolute paths, `..` traversal, drive letters, NTFS reserved
        // names. Do NOT replace with `entry.name()` or
        // `entry.mangled_name()`: both hand back paths like `../../x`.
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut f)?;
            // Keep the archived modified time, as Explorer and the stock
            // installer do: UE3 treats a Default*.ini whose mtime changed
            // as outdated (see dos_time).
            dos_time::apply_zip(&f, entry.last_modified(), &out);
        }
        sink.report("unzipping", i + 1, total, &out);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::sink;
    use super::*;

    #[test]
    fn extract_writes_expected_files() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("a.zip");
        {
            use std::io::Write;
            let f = std::fs::File::create(&zip_path).unwrap();
            let mut zw = ::zip::ZipWriter::new(f);
            let opts: ::zip::write::FileOptions<()> = ::zip::write::FileOptions::default();
            zw.start_file("hello.txt", opts).unwrap();
            zw.write_all(b"hi").unwrap();
            zw.start_file("nested/deep.txt", opts).unwrap();
            zw.write_all(b"deep").unwrap();
            zw.finish().unwrap();
        }
        let out = dir.path().join("out");
        let (sink, _rx) = sink();
        extract(&zip_path, &out, &sink).unwrap();
        assert_eq!(
            std::fs::read_to_string(out.join("hello.txt")).unwrap(),
            "hi"
        );
        assert_eq!(
            std::fs::read_to_string(out.join("nested/deep.txt")).unwrap(),
            "deep"
        );
    }

    // Bug shape: unzipped files kept the extraction time, so a seed zip of
    // the stock client made UE3 flag every Default*.ini as outdated. An
    // entry with no recorded time (the zip crate's 1980-01-01 placeholder)
    // keeps the extraction time rather than dating the file 1980.
    #[test]
    fn extract_keeps_the_archived_modified_time() {
        use super::super::test_fixtures::{fixture_mtime, FIXTURE_DOS_DATE, FIXTURE_DOS_TIME};
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("a.zip");
        {
            use std::io::Write;
            let f = std::fs::File::create(&zip_path).unwrap();
            let mut zw = ::zip::ZipWriter::new(f);
            let stamped: ::zip::write::FileOptions<()> = ::zip::write::FileOptions::default()
                .last_modified_time(
                    ::zip::DateTime::try_from_msdos(FIXTURE_DOS_DATE, FIXTURE_DOS_TIME).unwrap(),
                );
            zw.start_file("Config/DefaultEngine.ini", stamped).unwrap();
            zw.write_all(b"[Engine]").unwrap();
            let unstamped: ::zip::write::FileOptions<()> =
                ::zip::write::FileOptions::default().last_modified_time(::zip::DateTime::default());
            zw.start_file("unstamped.txt", unstamped).unwrap();
            zw.write_all(b"x").unwrap();
            zw.finish().unwrap();
        }
        let out = dir.path().join("out");
        let started = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        let (sink, _rx) = sink();
        extract(&zip_path, &out, &sink).unwrap();
        let mtime = |p: &str| std::fs::metadata(out.join(p)).unwrap().modified().unwrap();
        assert_eq!(mtime("Config/DefaultEngine.ini"), fixture_mtime());
        assert!(mtime("unstamped.txt") >= started);
    }
}
