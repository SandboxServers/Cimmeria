//! Source checks name the entries they refuse (launcher-adopt-ux AU-02).
use super::in_place_tests::Fixture;
use super::source;
use super::*;

/// A client root with `Working\Binaries\SGW.exe`, the layout a picked folder needs.
fn layout(root: &Path) {
    let binaries = root.join("Working").join("Binaries");
    std::fs::create_dir_all(&binaries).unwrap();
    std::fs::write(binaries.join("SGW.exe"), b"MZ").unwrap();
}

/// The write probes a check may leave in `dir`.
fn probes(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".cimmeria-write-probe-")
        })
        .count()
}

/// `working` is a junction to `target`. Returns false when `mklink` cannot run.
#[cfg(windows)]
fn junction(working: &Path, target: &Path) -> bool {
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(working)
        .arg(target)
        .status();
    matches!(made, Ok(status) if status.success())
}

/// `Working` is a junction to another folder. Both modes name the junction and
/// neither leaves a probe in the junction target.
#[cfg(windows)]
#[test]
fn a_junctioned_working_folder_is_named() {
    let f = Fixture::new(None);
    let target = tempfile::tempdir().unwrap();
    // The junction's `Binaries` is the target's own `Binaries`, so it is laid out flat.
    std::fs::create_dir_all(target.path().join("Binaries")).unwrap();
    std::fs::write(target.path().join("Binaries").join("SGW.exe"), b"MZ").unwrap();
    let other = tempfile::tempdir().unwrap();
    if !junction(&other.path().join("Working"), target.path()) {
        return;
    }
    for mode in [AdoptionMode::InPlace, AdoptionMode::Copy] {
        let request = SourceRequest::Folder {
            selected: other.path().to_path_buf(),
        };
        let Err(Error::Unsafe(report)) = source::request_source(&f.state, &request, mode) else {
            panic!("expected an unsafe report for {mode:?}");
        };
        assert_eq!(
            report.entries,
            [UnsafeEntry {
                reason: UnsafeReason::Link,
                path: Some("Working".into()),
            }]
        );
        assert_eq!(report.total, 1);
    }
    assert_eq!(probes(target.path()), 0);
    assert_eq!(probes(&target.path().join("Binaries")), 0);
}

/// A hard link inside a picked folder is named by both of its relative paths.
#[test]
fn a_hard_link_in_a_picked_folder_is_named() {
    let f = Fixture::new(None);
    let picked = tempfile::tempdir().unwrap();
    layout(picked.path());
    std::fs::write(picked.path().join("notes.txt"), b"same").unwrap();
    std::fs::hard_link(
        picked.path().join("notes.txt"),
        picked.path().join("notes-copy.txt"),
    )
    .unwrap();
    let request = SourceRequest::Folder {
        selected: picked.path().to_path_buf(),
    };
    let Err(Error::Unsafe(report)) = source::request_source(&f.state, &request, AdoptionMode::Copy)
    else {
        panic!("expected an unsafe report");
    };
    assert_eq!(
        report.entries,
        [
            UnsafeEntry {
                reason: UnsafeReason::HardLink,
                path: Some("notes-copy.txt".into()),
            },
            UnsafeEntry {
                reason: UnsafeReason::HardLink,
                path: Some("notes.txt".into()),
            },
        ]
    );
    assert_eq!(report.total, 2);
}

/// A source folder reached through a junction is not at its canonical path.
#[cfg(windows)]
#[test]
fn a_source_folder_through_a_junction_is_not_canonical() {
    let target = tempfile::tempdir().unwrap();
    let holder = tempfile::tempdir().unwrap();
    let link = holder.path().join("link");
    if !junction(&link, target.path()) {
        return;
    }
    let source = AdoptionSource::Folder(FolderSource {
        selected: link.clone(),
        root: link.clone(),
        scan_digest: [0; 32],
    });
    let Err(Error::Unsafe(report)) = source::canonical_paths(&source) else {
        panic!("expected an unsafe report");
    };
    assert_eq!(
        report.entries,
        [UnsafeEntry {
            reason: UnsafeReason::Link,
            path: None,
        }]
    );
    assert_eq!(report.total, 1);
}
