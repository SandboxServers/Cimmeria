//! Zip extraction (seed and patch zips published by operators).

use std::path::Path;

use cimmeria_patchset::{ApplyOptions, SkippedOp};

use super::{dos_time, UnpackError, UnpackSink};

fn patchset_err(e: cimmeria_patchset::PatchsetError) -> UnpackError {
    use cimmeria_patchset::PatchsetError as P;
    let message = e.to_string();
    let (kind, path, expected, actual) = match e {
        P::SourceMismatch {
            path,
            expected,
            actual,
        } => ("source_mismatch", path, expected, actual),
        P::PatchOutputMismatch {
            path,
            expected,
            actual,
            ..
        } => ("patch_output_mismatch", path, expected, actual),
        P::ResultMismatch {
            path,
            expected,
            actual,
        } => ("result_mismatch", path, expected, actual),
        _ => return UnpackError::Patchset(message),
    };
    UnpackError::PatchsetHash {
        kind,
        path,
        expected,
        actual,
        message,
    }
}

/// Extract every entry of `zip_path` into `dest`, overwriting existing
/// files. Entries whose names would escape `dest` are skipped by
/// `enclosed_name`.
///
/// A zip carrying a `cimmeria-patch.json` recipe is a patch set: it is
/// applied with `cimmeria-patchset`, which rebuilds files from the stock
/// ones in `dest` and then writes the overlay entries. `opts` goes to the
/// apply; the returned vec holds the ops it skipped (empty for a plain zip).
pub(super) fn extract(
    zip_path: &Path,
    dest: &Path,
    sink: &UnpackSink,
    opts: &ApplyOptions,
) -> Result<Vec<SkippedOp>, UnpackError> {
    if cimmeria_patchset::apply::has_recipe(zip_path).map_err(patchset_err)? {
        sink.check_cancel()?;
        let mut n = 0;
        let report = cimmeria_patchset::apply_with(zip_path, dest, opts, &mut |name| {
            n += 1;
            sink.report("patching", n, n, &dest.join(name));
        })
        .map_err(patchset_err)?;
        tracing::info!(
            rebuilt = report.rebuilt.len(),
            already_current = report.already_current.len(),
            overlay_files = report.overlay_files,
            skipped = report.skipped.len(),
            "applied patch set"
        );
        return Ok(report.skipped);
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
    Ok(Vec::new())
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
        extract(&zip_path, &out, &sink, &ApplyOptions::default()).unwrap();
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
        extract(&zip_path, &out, &sink, &ApplyOptions::default()).unwrap();
        let mtime = |p: &str| std::fs::metadata(out.join(p)).unwrap().modified().unwrap();
        assert_eq!(mtime("Config/DefaultEngine.ini"), fixture_mtime());
        assert!(mtime("unstamped.txt") >= started);
    }

    /// A stock-source mismatch keeps its path and both hashes for the
    /// install-result telemetry event, and the same message text as
    /// every other patch-set error.
    #[test]
    fn a_source_mismatch_keeps_its_path_and_hashes() {
        let e = patchset_err(cimmeria_patchset::PatchsetError::SourceMismatch {
            path: "SGWGame/CookedPC/x.upk".into(),
            expected: "aa".into(),
            actual: "bb".into(),
        });
        match &e {
            UnpackError::PatchsetHash {
                kind,
                path,
                expected,
                actual,
                ..
            } => {
                assert_eq!(
                    (*kind, path.as_str(), expected.as_str(), actual.as_str()),
                    ("source_mismatch", "SGWGame/CookedPC/x.upk", "aa", "bb")
                );
            }
            other => panic!("expected PatchsetHash, got {other:?}"),
        }
        assert!(e
            .to_string()
            .starts_with("Patch set error: SGWGame/CookedPC/x.upk does not match"));
        let other = patchset_err(cimmeria_patchset::PatchsetError::Invalid("bad".into()));
        assert!(matches!(other, UnpackError::Patchset(m) if m == "bad"));
    }

    /// A recipe zip with one op: `DIALOG` rebuilt from its stock bytes.
    fn one_op_recipe_zip(dir: &Path) -> std::path::PathBuf {
        const DIALOG: &str = "Working/SGWGame/UI/Dialog.lua";
        let (stock, patched, spec_dir) = (dir.join("stock"), dir.join("patched"), dir.join("spec"));
        for root in [&stock, &patched] {
            std::fs::create_dir_all(root.join("Working/SGWGame/UI")).unwrap();
        }
        std::fs::create_dir_all(&spec_dir).unwrap();
        let stock_bytes = b"function Dialog.show()\r\nend\r\n".repeat(40);
        let mut patched_bytes = stock_bytes.clone();
        patched_bytes.extend_from_slice(b"-- ours\r\n");
        std::fs::write(stock.join(DIALOG), &stock_bytes).unwrap();
        std::fs::write(patched.join(DIALOG), &patched_bytes).unwrap();
        let spec: cimmeria_patchset::Spec = serde_json::from_str(&format!(
            r#"{{"id":"one-op","ops":[{{"target":"{DIALOG}","sources":[{{"path":"{DIALOG}"}}]}}]}}"#
        ))
        .unwrap();
        let built = cimmeria_patchset::build(&spec, &spec_dir, &stock, &patched).unwrap();
        let zip_path = dir.join("recipe.zip");
        std::fs::write(&zip_path, &built.zip).unwrap();
        zip_path
    }

    // A recipe zip whose source was modded: tolerant mode skips that one op
    // and returns it, strict mode fails with the same error as before.
    #[test]
    fn tolerant_unpack_returns_the_skipped_op_and_strict_still_fails() {
        use super::super::{test_fixtures::sink, unpack_with};
        use cimmeria_patchset::{ApplyOptions, SkipReason, SkippedOp};
        let dir = tempfile::tempdir().unwrap();
        let zip_path = one_op_recipe_zip(dir.path());
        let dest = dir.path().join("install");
        std::fs::create_dir_all(dest.join("Working/SGWGame/UI")).unwrap();
        std::fs::write(dest.join("Working/SGWGame/UI/Dialog.lua"), b"modded").unwrap();
        let (sink, _rx) = sink();
        let tolerant = ApplyOptions {
            tolerate_ops: true,
            only_targets: None,
        };
        let skipped = unpack_with(&zip_path, &dest, &sink, &tolerant).unwrap();
        assert_eq!(
            skipped,
            [SkippedOp {
                target: "Working/SGWGame/UI/Dialog.lua".into(),
                reason: SkipReason::SourceMismatch,
            }]
        );
        assert!(matches!(
            unpack_with(&zip_path, &dest, &sink, &ApplyOptions::default()),
            Err(super::super::UnpackError::PatchsetHash {
                kind: "source_mismatch",
                ..
            })
        ));
    }
}
