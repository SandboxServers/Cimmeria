use super::survey::*;
use super::*;

/// A temp folder, with its path canonicalized for the survey.
fn folder() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    (dir, root)
}

/// Two names for one file, `a` and `b`, inside `root`.
fn hard_pair(root: &Path, a: &str, b: &str) {
    std::fs::write(root.join(a), b"same").unwrap();
    std::fs::hard_link(root.join(a), root.join(b)).unwrap();
}

fn entry(reason: UnsafeReason, path: &str) -> UnsafeEntry {
    UnsafeEntry {
        reason,
        path: Some(path.to_owned()),
    }
}

#[test]
fn reparse_tags_map_to_reasons() {
    for tag in [0xA000_0003, 0xA000_000C] {
        assert_eq!(inventory::reparse_reason(tag), UnsafeReason::Link);
    }
    for tag in [
        0x9000_001A,
        0x9000_101A,
        0x9000_701A,
        0x9000_F01A,
        0x8000_0021,
    ] {
        assert_eq!(inventory::reparse_reason(tag), UnsafeReason::CloudFile);
    }
    for tag in [0x8000_0017, 0x8000_001B] {
        assert_eq!(inventory::reparse_reason(tag), UnsafeReason::SpecialFile);
    }
}

#[test]
fn a_clean_folder_has_nothing_to_explain() {
    let (_dir, root) = folder();
    std::fs::write(root.join("a.txt"), b"a").unwrap();
    std::fs::create_dir(root.join("sub")).unwrap();
    std::fs::write(root.join("sub").join("b.txt"), b"b").unwrap();
    let report = survey(&root);
    assert_eq!(report.total, 0);
    assert!(report.entries.is_empty());
    assert_eq!(explain(&root), Error::Storage(StorageError::UnsafeFile));
}

#[test]
fn hard_links_are_named_by_relative_path() {
    let (_dir, root) = folder();
    std::fs::write(root.join("a.txt"), b"a").unwrap();
    hard_pair(&root, "other.txt", "later.txt");
    let expected = UnsafeReport {
        entries: vec![
            entry(UnsafeReason::HardLink, "later.txt"),
            entry(UnsafeReason::HardLink, "other.txt"),
        ],
        total: 2,
    };
    assert_eq!(survey(&root), expected);
    assert_eq!(explain(&root), Error::Unsafe(Box::new(expected)));
}

#[test]
fn the_report_lists_five_and_counts_all() {
    let (_dir, root) = folder();
    for i in 0..7 {
        hard_pair(&root, &format!("f{i}.txt"), &format!("g{i}.txt"));
    }
    let report = survey(&root);
    assert_eq!(report.entries.len(), 5);
    assert_eq!(report.total, 14);
    assert!(report
        .entries
        .iter()
        .all(|entry| entry.reason == UnsafeReason::HardLink));
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn an_offender_is_not_descended() {
    // A control character is valid on Unix only; Windows refuses it in a name,
    // and `a_control_character_name_is_refused` covers the rule on every host.
    let (_dir, root) = folder();
    let dir = root.join("bad\u{7}");
    std::fs::create_dir(&dir).unwrap();
    hard_pair(&dir, "x.txt", "y.txt");
    let report = survey(&root);
    assert_eq!(
        report,
        UnsafeReport {
            entries: vec![entry(UnsafeReason::Name, "bad\u{7}")],
            total: 1,
        }
    );
}

#[test]
fn a_non_ascii_stock_name_is_scanned() {
    let (_dir, root) = folder();
    let name = "Working/Binaries/wxRes/TerrainEdit_Gradient másolata.bmp";
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"same").unwrap();
    let cancel = CancellationToken::new();
    let index = inventory::scan(&root, &cancel).unwrap();
    let expected = inventory::hash(&path, &cancel).unwrap();
    let found = &index[name];
    assert_eq!(found.size, 4);
    assert_eq!(found.sha256, expected.sha256);
}

#[test]
fn a_non_ascii_name_is_not_reported() {
    let (_dir, root) = folder();
    let dir = root.join("Working").join("wxRes");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("TerrainEdit_Gradient másolata.bmp"), b"same").unwrap();
    let report = survey(&root);
    assert_eq!(report.total, 0);
    assert!(report.entries.is_empty());
}

#[test]
fn a_control_character_name_is_refused() {
    assert!(!inventory::scan_name_allowed("bad\u{7}.bmp"));
    assert!(inventory::scan_name_allowed(
        "Working/Binaries/wxRes/TerrainEdit_Gradient másolata.bmp"
    ));
}
#[cfg(windows)]
#[test]
fn a_junction_is_a_link() {
    // The link is made at the raw temp path: `cmd` does not take the verbatim
    // path that `canonicalize` returns on Windows.
    let (dir, root) = folder();
    let target = tempfile::tempdir().unwrap();
    hard_pair(target.path(), "x.txt", "y.txt");
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(dir.path().join("Working"))
        .arg(target.path())
        .status();
    if !matches!(made, Ok(status) if status.success()) {
        return;
    }
    let working = root.join("Working");
    assert_eq!(
        survey(&root),
        UnsafeReport {
            entries: vec![entry(UnsafeReason::Link, "Working")],
            total: 1,
        }
    );
    assert_eq!(
        survey(&working),
        UnsafeReport {
            entries: vec![entry(UnsafeReason::Link, ".")],
            total: 1,
        }
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_folder_is_a_link() {
    let (_dir, root) = folder();
    let target = tempfile::tempdir().unwrap();
    hard_pair(target.path(), "x.txt", "y.txt");
    std::os::unix::fs::symlink(target.path(), root.join("Working")).unwrap();
    assert_eq!(
        survey(&root),
        UnsafeReport {
            entries: vec![entry(UnsafeReason::Link, "Working")],
            total: 1,
        }
    );
    assert_eq!(
        survey(&root.join("Working")),
        UnsafeReport {
            entries: vec![entry(UnsafeReason::Link, ".")],
            total: 1,
        }
    );
}

#[cfg(target_os = "linux")]
#[test]
fn sockets_and_case_collisions() {
    let (_dir, root) = folder();
    let _listener = std::os::unix::net::UnixListener::bind(root.join("sock")).unwrap();
    std::fs::write(root.join("README.txt"), b"a").unwrap();
    std::fs::write(root.join("Readme.txt"), b"b").unwrap();
    // Walk order is by name, so `README.txt` is kept and `Readme.txt` collides.
    assert_eq!(
        survey(&root),
        UnsafeReport {
            entries: vec![
                entry(UnsafeReason::CaseCollision, "Readme.txt"),
                entry(UnsafeReason::SpecialFile, "sock"),
            ],
            total: 2,
        }
    );
}

#[test]
fn shown_cuts_a_long_path() {
    // MAX_PATH_SHOWN is 200 chars, then the `…` marker.
    let long = "a".repeat(201);
    let cut = shown(Path::new(&long));
    assert!(cut.ends_with('…'));
    assert_eq!(cut.chars().count(), 201);
    let short = "a".repeat(10);
    assert_eq!(shown(Path::new(&short)), short);
}
