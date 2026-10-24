use super::fixture::{write_cab_set, CabOptions};
use super::*;
use crate::unpack::test_fixtures::{fixture_mtime, incompressible, sink};

fn install_path(dest: &Path, name: &str) -> PathBuf {
    dest.join(name.replace('\\', "/"))
}

/// Text-like bytes with long repeats, so the encoder's matches reach across
/// block and cabinet boundaries.
fn repetitive(seed: u8, len: usize) -> Vec<u8> {
    let phrase = format!("[Engine.Core{seed}] Value=0x{seed:02X} ; the same line again\r\n");
    phrase.bytes().cycle().take(len).collect()
}

fn expand(
    dir: &Path,
    names: &[String],
    total: usize,
) -> Result<(PathBuf, Vec<crate::install::Progress>), UnpackError> {
    let dest = dir.join("install");
    let (sink, mut rx) = sink();
    expand_chain(dir, names, &dest, total, &sink)?;
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    Ok((dest, events))
}

// The bug shape this guards: a file that starts in DATA1.CAB and ends in
// DATA2.CAB (or DATA3.CAB) must come out whole, MSZIP history must survive
// block and cabinet boundaries, and a block split across two cabinets must be
// joined, with nothing written twice or dropped at a boundary.
#[test]
fn expands_a_spanning_set_with_split_blocks_and_cross_block_history() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![
        ("Working\\binaries\\SGW.exe", incompressible(1, 150_000)),
        (
            "Working\\SGWGame\\Config\\DefaultEngine.ini",
            repetitive(2, 90_000),
        ),
        ("Common\\res\\x.def", b"<root/>".to_vec()),
        ("Working\\SGWGame\\CookedPC\\a.upk", repetitive(3, 70_000)),
    ];
    let info = write_cab_set(dir.path(), &files, CabOptions::default());
    assert!(info.cabinets.len() >= 3, "{:?}", info.cabinets);
    assert!(info.cross_block_matches > 0, "fixture must reach back");

    let (dest, events) = expand(dir.path(), &info.cabinets, files.len()).unwrap();
    for (name, data) in &files {
        assert_eq!(
            &std::fs::read(install_path(&dest, name)).unwrap(),
            data,
            "{name}"
        );
    }
    assert_eq!(events.len(), files.len(), "one progress event per file");
}

#[test]
fn expands_unsplit_boundaries_stored_folders_and_several_folders() {
    for opts in [
        CabOptions {
            split_blocks: false,
            ..CabOptions::default()
        },
        CabOptions {
            compression: 0,
            ..CabOptions::default()
        },
        CabOptions {
            files_per_folder: 1,
            blocks_per_cab: 1000,
            ..CabOptions::default()
        },
    ] {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![
            ("a.bin", incompressible(5, 70_000)),
            ("b.txt", repetitive(6, 50_000)),
            ("c.txt", repetitive(7, 20_000)),
        ];
        let info = write_cab_set(dir.path(), &files, opts);
        let (dest, _) = expand(dir.path(), &info.cabinets, files.len()).unwrap();
        for (name, data) in &files {
            assert_eq!(&std::fs::read(dest.join(name)).unwrap(), data, "{opts:?}");
        }
    }
}

#[test]
fn zero_length_files_are_created_anywhere_in_a_folder() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![
        ("empty-first.txt", Vec::new()),
        ("data.bin", repetitive(8, 40_000)),
        ("Working\\editor\\DO_NOT_DELETE.txt", Vec::new()),
        ("more.bin", incompressible(9, 30_000)),
        ("empty-last.txt", Vec::new()),
    ];
    let info = write_cab_set(dir.path(), &files, CabOptions::default());
    let (dest, _) = expand(dir.path(), &info.cabinets, files.len()).unwrap();
    for (name, data) in &files {
        assert_eq!(
            &std::fs::read(install_path(&dest, name)).unwrap(),
            data,
            "{name}"
        );
    }
}

// Bug shape: expanded files kept the extraction time instead of the
// cabinet's 2009-06-30 stamp, and UE3 then asked to regenerate every ini.
#[test]
fn expanded_files_keep_the_cabinet_date_and_time() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![
        (
            "Working\\SGWGame\\Config\\DefaultEditor.ini",
            b"[Editor]\r\n".to_vec(),
        ),
        ("Working\\binaries\\SGW.exe", incompressible(4, 150_000)),
    ];
    let info = write_cab_set(dir.path(), &files, CabOptions::default());
    assert!(info.cabinets.len() >= 2);
    let (dest, _) = expand(dir.path(), &info.cabinets, files.len()).unwrap();
    for (name, _) in &files {
        let modified = std::fs::metadata(install_path(&dest, name))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(modified, fixture_mtime(), "{name}");
    }
}

#[test]
fn rejects_case_conflicts_and_unsafe_names_before_creating_output() {
    for (second, unsafe_name) in [("A.txt", false), ("..\\escape.txt", true)] {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![("a.txt", b"a".to_vec()), (second, b"b".to_vec())];
        let info = write_cab_set(dir.path(), &files, CabOptions::default());
        let err = expand(dir.path(), &info.cabinets, 2).unwrap_err();
        if unsafe_name {
            assert!(matches!(err, UnpackError::UnsafePath(_)), "{err:?}");
        } else {
            assert!(matches!(err, UnpackError::EntryConflict(_)), "{err:?}");
        }
        assert!(!dir.path().join("install").exists());
    }
}

#[test]
fn rejects_an_index_count_mismatch_before_creating_output() {
    let dir = tempfile::tempdir().unwrap();
    let info = write_cab_set(
        dir.path(),
        &[("a.txt", b"a".to_vec())],
        CabOptions::default(),
    );
    let err = expand(dir.path(), &info.cabinets, 2).unwrap_err();
    assert!(matches!(err, UnpackError::Cab(_)), "{err:?}");
    assert!(!dir.path().join("install").exists());
}

#[test]
fn a_missing_or_reordered_cabinet_fails() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![("big.bin", incompressible(3, 250_000))];
    let info = write_cab_set(dir.path(), &files, CabOptions::default());
    assert!(info.cabinets.len() >= 3, "{:?}", info.cabinets);

    let mut swapped = info.cabinets.clone();
    swapped.swap(1, 2);
    let err = expand(dir.path(), &swapped, 1).unwrap_err();
    assert!(matches!(err, UnpackError::Cab(_)), "{err:?}");

    let mut skipped = info.cabinets.clone();
    skipped.remove(1);
    let err = expand(dir.path(), &skipped, 1).unwrap_err();
    assert!(matches!(err, UnpackError::Cab(_)), "{err:?}");

    std::fs::remove_file(dir.path().join(&info.cabinets[1])).unwrap();
    let err = expand(dir.path(), &info.cabinets, 1).unwrap_err();
    assert!(matches!(err, UnpackError::Cab(_)), "{err:?}");
}

#[test]
fn a_set_that_stops_mid_file_fails() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![("big.bin", incompressible(3, 250_000))];
    let info = write_cab_set(dir.path(), &files, CabOptions::default());
    let first_two = &info.cabinets[..2];
    let err = expand(dir.path(), first_two, 1).unwrap_err();
    assert!(matches!(err, UnpackError::Cab(_)), "{err:?}");
}

#[test]
fn corrupt_compressed_data_fails_instead_of_writing_garbage() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![("a.txt", repetitive(1, 60_000))];
    let info = write_cab_set(dir.path(), &files, CabOptions::default());
    let path = dir.path().join(&info.cabinets[0]);
    let mut bytes = std::fs::read(&path).unwrap();
    // Clobber the middle of the first cabinet's data.
    let mid = bytes.len() - bytes.len() / 3;
    for b in &mut bytes[mid..mid + 64] {
        *b = 0xFF;
    }
    std::fs::write(&path, bytes).unwrap();
    let err = expand(dir.path(), &info.cabinets, 1).unwrap_err();
    assert!(matches!(err, UnpackError::Cab(_)), "{err:?}");
}

#[test]
fn lzx_and_quantum_folders_are_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let info = write_cab_set(
        dir.path(),
        &[("a.txt", b"abc".to_vec())],
        CabOptions {
            compression: 3,
            ..CabOptions::default()
        },
    );
    match expand(dir.path(), &info.cabinets, 1).unwrap_err() {
        UnpackError::Cab(message) => assert!(message.contains("Lzx"), "{message}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_file_that_is_not_a_cabinet_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("DATA1.CAB"), b"PK\x03\x04 not a cabinet").unwrap();
    let err = expand(dir.path(), &["DATA1.CAB".into()], 1).unwrap_err();
    assert!(matches!(err, UnpackError::Cab(_)), "{err:?}");
}

#[test]
fn cancel_stops_before_the_first_file() {
    let dir = tempfile::tempdir().unwrap();
    let info = write_cab_set(
        dir.path(),
        &[("a.txt", b"a".to_vec())],
        CabOptions::default(),
    );
    let (sink, _rx) = sink();
    sink.cancel.cancel();
    let out = dir.path().join("out");
    let err = expand_chain(dir.path(), &info.cabinets, &out, 1, &sink).unwrap_err();
    assert!(matches!(err, UnpackError::Cancelled), "{err:?}");
    assert!(!out.join("a.txt").exists());
}

#[test]
fn the_written_inf_lists_the_set_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![("big.bin", incompressible(3, 250_000))];
    let info = write_cab_set(dir.path(), &files, CabOptions::default());
    let set = super::super::cab_set::find_installer(dir.path())
        .unwrap()
        .unwrap();
    assert_eq!(set.cabinets, info.cabinets);
    assert_eq!(set.file_count, 1);
}

// On Windows, the portable decoder must agree byte for byte with FDI on a
// set MakeCAB itself wrote (real MSZIP history, real split blocks).
#[cfg(windows)]
#[test]
fn matches_fdi_on_a_makecab_spanning_set() {
    use crate::unpack::test_fixtures::make_cab_set;
    let dir = tempfile::tempdir().unwrap();
    let cabs = dir.path().join("Data");
    std::fs::create_dir_all(&cabs).unwrap();
    let files = vec![
        ("Working\\binaries\\SGW.exe", incompressible(1, 150_000)),
        (
            "Working\\SGWGame\\Config\\Default.ini",
            repetitive(2, 120_000),
        ),
        ("Common\\res\\x.def", b"<root/>".to_vec()),
    ];
    let names = make_cab_set(&cabs, &files, 100_000);
    assert!(names.len() >= 2, "{names:?}");
    let (sink, _rx) = sink();
    let portable = dir.path().join("portable");
    expand_chain(&cabs, &names, &portable, files.len(), &sink).unwrap();
    let fdi = dir.path().join("fdi");
    crate::unpack::fdi::expand_chain(&cabs, &names, &fdi, files.len(), &sink).unwrap();
    for (name, data) in &files {
        let a = std::fs::read(install_path(&portable, name)).unwrap();
        assert_eq!(
            a,
            std::fs::read(install_path(&fdi, name)).unwrap(),
            "{name}"
        );
        assert_eq!(&a, data, "{name}");
    }
}
