//! Zip extraction (seed and patch zips published by operators).

use std::path::Path;

use super::{UnpackError, UnpackSink};

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
}
