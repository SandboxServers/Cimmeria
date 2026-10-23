//! Source checks: the picked folder, the saved import and the write probe.
use super::in_place_tests::Fixture;
use super::source::*;
use super::*;

#[test]
fn copy_of_a_folder_source_is_admitted_without_write_access() {
    let f = Fixture::new(None);
    // A game folder that is not the saved import: a folder source, copied
    // read only, so it is admitted even when it is not writable.
    let picked = tempfile::tempdir().unwrap();
    layout(picked.path());
    let request = SourceRequest::Folder {
        selected: picked.path().to_path_buf(),
    };
    let (adopted, _) = request_source(&f.state, &request, AdoptionMode::Copy).unwrap();
    assert!(matches!(adopted, AdoptionSource::Folder(_)));
}

/// A minimal PE header: enough for the executable to be recognised.
fn sgw_exe() -> Vec<u8> {
    let mut exe = vec![0u8; 0x200];
    exe[..2].copy_from_slice(b"MZ");
    exe[60..64].copy_from_slice(&0x128u32.to_le_bytes());
    exe[0x128..0x12c].copy_from_slice(b"PE\0\0");
    exe[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
    exe[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
    exe[0x186..0x188].copy_from_slice(&0x40u16.to_le_bytes());
    exe
}

/// Lays `root` out as the installer does: `Working\Binaries\SGW.exe`.
fn layout(root: &Path) {
    let binaries = root.join("Working").join("Binaries");
    std::fs::create_dir_all(&binaries).unwrap();
    std::fs::write(binaries.join("SGW.exe"), sgw_exe()).unwrap();
}

#[test]
fn executable_pick_resolves_one_install_in_place_without_touching_any_file() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["Live", "Test"] {
        layout(&dir.path().join(name));
    }
    let root = dir.path().join("Test");
    let selected = root.join("Working/Binaries/SGW.exe");
    let before = std::fs::read(&selected).unwrap();
    assert_eq!(
        resolve_executable(&selected).unwrap(),
        root.canonicalize().unwrap()
    );
    assert_eq!(std::fs::read(&selected).unwrap(), before);
    assert!(dir.path().join("Live/Working/Binaries/SGW.exe").is_file());
    assert!(!root.join("game").exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn executable_pick_keeps_the_selected_outer_install_when_binaries_holds_another_layout() {
    let dir = tempfile::tempdir().unwrap();
    layout(dir.path());
    layout(&dir.path().join("Working/Binaries"));
    let selected = dir.path().join("Working/Binaries/SGW.exe");
    assert_eq!(
        resolve_executable(&selected).unwrap(),
        dir.path().canonicalize().unwrap()
    );
    let nested = dir.path().join("Working/Binaries/Working/Binaries/SGW.exe");
    assert_eq!(
        resolve_executable(&nested).unwrap(),
        dir.path().join("Working/Binaries").canonicalize().unwrap()
    );
    assert!(selected.is_file());
    assert!(nested.is_file());
}

#[test]
fn executable_pick_refuses_other_names_directories_missing_files_and_flat_layouts() {
    let dir = tempfile::tempdir().unwrap();
    layout(dir.path());
    let binaries = dir.path().join("Working/Binaries");
    let wrong = binaries.join("Launcher.exe");
    std::fs::write(&wrong, sgw_exe()).unwrap();
    assert!(resolve_executable(&wrong).is_err());
    assert!(resolve_executable(&binaries).is_err());
    assert!(resolve_executable(&dir.path().join("missing/SGW.exe")).is_err());
    let flat = dir.path().join("SGW.exe");
    std::fs::write(&flat, sgw_exe()).unwrap();
    assert!(resolve_executable(&flat).is_err());
}

#[test]
#[cfg(unix)]
fn executable_pick_refuses_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    layout(dir.path());
    let exe = dir.path().join("Working/Binaries/SGW.exe");
    std::fs::rename(&exe, exe.with_extension("original")).unwrap();
    std::os::unix::fs::symlink(exe.with_extension("original"), &exe).unwrap();
    assert!(resolve_executable(&exe).is_err());
}

#[test]
#[cfg(unix)]
fn executable_pick_refuses_linked_game_layout_before_resolving_its_root() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    layout(&real);
    let linked = dir.path().join("linked");
    std::fs::create_dir(&linked).unwrap();
    std::os::unix::fs::symlink(real.join("Working"), linked.join("Working")).unwrap();
    assert!(resolve_executable(&linked.join("Working/Binaries/SGW.exe")).is_err());
}

#[test]
fn picked_root_working_or_binaries_resolve_to_one_root() {
    let dir = tempfile::tempdir().unwrap();
    layout(dir.path());
    let root = dir.path().canonicalize().unwrap();
    for picked in [
        root.clone(),
        root.join("Working"),
        root.join("Working").join("Binaries"),
    ] {
        assert_eq!(resolve_folder(&picked).unwrap(), root);
    }
}

#[test]
fn folder_without_sgw_is_not_a_game_folder() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Working")).unwrap();
    let (result, record) = rejection::scope(|| resolve_folder(dir.path()));
    assert!(matches!(result, Err(Error::NotAGameFolder)));
    let record = record.expect("a recorded rejection");
    assert_eq!(record.check, "not_a_game_folder");
    assert_eq!(record.entry.as_deref(), Some("."));
    assert_eq!(
        record.expected.as_deref(),
        Some("SGW.exe and the game folders")
    );
}

#[test]
fn flat_folder_is_not_a_game_folder_in_either_mode() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("SGW.exe"), sgw_exe()).unwrap();
    std::fs::create_dir(dir.path().join("SGWGame")).unwrap();
    assert!(matches!(
        resolve_folder(dir.path()),
        Err(Error::NotAGameFolder)
    ));
}

#[test]
fn flat_folder_in_copy_mode_is_not_a_game_folder() {
    let f = Fixture::new(None);
    let picked = tempfile::tempdir().unwrap();
    std::fs::write(picked.path().join("SGW.exe"), sgw_exe()).unwrap();
    std::fs::create_dir(picked.path().join("SGWGame")).unwrap();
    let request = SourceRequest::Folder {
        selected: picked.path().to_path_buf(),
    };
    let (result, record) =
        rejection::scope(|| request_source(&f.state, &request, AdoptionMode::Copy));
    assert!(matches!(result, Err(Error::NotAGameFolder)));
    assert_eq!(record.map(|r| r.check), Some("not_a_game_folder"));
}

#[test]
fn the_one_game_folder_inside_a_pick_is_picked() {
    let dir = tempfile::tempdir().unwrap();
    layout(&dir.path().join("Stargate"));
    std::fs::create_dir(dir.path().join("Downloads")).unwrap();
    let root = dir.path().join("Stargate").canonicalize().unwrap();
    assert_eq!(resolve_folder(dir.path()).unwrap(), root);
}

#[test]
fn two_game_folders_inside_a_pick_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    layout(&dir.path().join("Live"));
    layout(&dir.path().join("Test"));
    let (result, record) = rejection::scope(|| resolve_folder(dir.path()));
    assert!(matches!(result, Err(Error::NotAGameFolder)));
    let record = record.expect("a recorded rejection");
    assert_eq!(record.check, "several_game_folders");
    assert_eq!(record.found.as_deref(), Some("2"));
}

/// `link` is a junction to `target`. Returns false when `mklink` cannot run.
#[cfg(windows)]
fn junction_to(link: &Path, target: &Path) -> bool {
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status();
    matches!(made, Ok(status) if status.success())
}

/// A junction is a link, so `child_roots` skips it: a pick holding one real game
/// folder and a junction to another layout still resolves to the real one.
#[cfg(windows)]
#[test]
fn a_junction_child_is_skipped_and_not_a_second_game_folder() {
    let dir = tempfile::tempdir().unwrap();
    layout(&dir.path().join("Real"));
    let target = tempfile::tempdir().unwrap();
    layout(target.path());
    if !junction_to(&dir.path().join("Alias"), target.path()) {
        return;
    }
    let root = dir.path().join("Real").canonicalize().unwrap();
    assert_eq!(resolve_folder(dir.path()).unwrap(), root);
}

#[test]
fn saved_import_naming_the_folder_is_legacy_another_is_folder() {
    let f = Fixture::new(None);
    let named = SourceRequest::Folder {
        selected: f.folder().to_path_buf(),
    };
    layout(f.folder());
    let (adopted, _) = request_source(&f.state, &named, AdoptionMode::InPlace).unwrap();
    assert!(matches!(adopted, AdoptionSource::Legacy(_)));
    let other = tempfile::tempdir().unwrap();
    layout(other.path());
    let request = SourceRequest::Folder {
        selected: other.path().to_path_buf(),
    };
    let (adopted, _) = request_source(&f.state, &request, AdoptionMode::InPlace).unwrap();
    assert!(matches!(adopted, AdoptionSource::Folder(_)));
}

/// Restores the write permission the test denies, even when it fails.
#[cfg(windows)]
struct RestoreWrite {
    user: String,
    folder: PathBuf,
}

#[cfg(windows)]
impl Drop for RestoreWrite {
    fn drop(&mut self) {
        let _ = std::process::Command::new("icacls")
            .arg(&self.folder)
            .arg("/remove:d")
            .arg(&self.user)
            .status();
    }
}

#[cfg(windows)]
#[test]
fn denied_write_is_folder_not_writable() {
    let dir = tempfile::tempdir().unwrap();
    layout(dir.path());
    // `(WD)` only refuses file creation: a `(W)` deny also blocks listing
    // the folder, which hides `SGW.exe` before the probe runs.
    let binaries = dir.path().join("Working").join("Binaries");
    let user = std::env::var("USERNAME").unwrap();
    let status = std::process::Command::new("icacls")
        .arg(&binaries)
        .arg("/deny")
        .arg(format!("{user}:(WD)"))
        .status()
        .unwrap();
    assert!(status.success());
    let _restore = RestoreWrite {
        user,
        folder: binaries,
    };
    let (result, record) = rejection::scope(|| check_writable(dir.path()));
    assert!(matches!(result, Err(Error::FolderNotWritable)));
    let record = record.expect("a recorded rejection");
    assert_eq!(record.check, "folder_not_writable");
    assert_eq!(record.expected.as_deref(), Some("writable"));
    assert!(record.found.is_some(), "the I/O kind is named");
}

/// Light moves files inside the picked folder like in place does, so a folder it
/// cannot write to is refused up front; a copy only reads and still goes through.
#[cfg(windows)]
#[test]
fn light_refuses_an_unwritable_folder_and_copy_does_not() {
    let f = Fixture::new(None);
    let dir = tempfile::tempdir().unwrap();
    layout(dir.path());
    let binaries = dir.path().join("Working").join("Binaries");
    let user = std::env::var("USERNAME").unwrap();
    let status = std::process::Command::new("icacls")
        .arg(&binaries)
        .arg("/deny")
        .arg(format!("{user}:(WD)"))
        .status()
        .unwrap();
    assert!(status.success());
    let _restore = RestoreWrite {
        user,
        folder: binaries,
    };
    let request = SourceRequest::Folder {
        selected: dir.path().to_path_buf(),
    };
    let light = request_source(&f.state, &request, AdoptionMode::Light);
    assert!(
        matches!(light, Err(Error::FolderNotWritable)),
        "expected FolderNotWritable for Light"
    );
    assert!(request_source(&f.state, &request, AdoptionMode::Copy).is_ok());
}
#[test]
fn import_blocker_is_silent_and_refuse_blocker_records_the_refusal() {
    let f = Fixture::new(None);
    let mut imported = f.state.lock().unwrap().legacy_import().unwrap().unwrap();
    imported.config.manifest_url = "https://example.invalid/manifest.json".into();
    // A status query asks for the blocker only: it refuses nothing, so it records nothing.
    let (blocker, record) = rejection::scope(|| import_blocker(&imported));
    assert!(matches!(blocker, Some(Error::UnsupportedCatalog)));
    assert!(record.is_none());
    let (refused, record) = rejection::scope(|| refuse_blocker(&imported));
    assert!(matches!(refused, Some(Error::UnsupportedCatalog)));
    assert_eq!(record.map(|r| r.check), Some("unsupported_catalog"));
}

#[test]
fn a_saved_import_the_request_does_not_name_is_import_changed() {
    let f = Fixture::new(None);
    let saved = f.state.lock().unwrap().legacy_import().unwrap();
    let request = SourceRequest::Legacy {
        import_digest: "not the confirmed import".into(),
    };
    let (result, record) = rejection::scope(|| saved_import(saved, &request));
    assert!(matches!(result, Err(Error::SourceChanged)));
    assert_eq!(record.map(|r| r.check), Some("import_changed"));
}
