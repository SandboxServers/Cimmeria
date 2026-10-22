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
