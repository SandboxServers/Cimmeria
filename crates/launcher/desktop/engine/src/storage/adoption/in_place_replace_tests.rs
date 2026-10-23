//! In-place adoption that replaces a modified release file with consent, and
//! keeps the player's original in the backup folder (AI-11).
use super::in_place_tests::*;
use super::*;
use crate::install_worker::fixtures;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::io::Write;

const EXE: &str = "Working/Binaries/SGW.exe";
const EDITED: &[u8] = b"patched by the player";
const EXTRA: &[u8] = b"extra entry";

/// `fixtures::archive(true)` with one more release file, `extra.txt`, so that one
/// file can go missing while the others stay reusable.
fn three_file_archive() -> Vec<u8> {
    archive_with("extra.txt", EXTRA)
}

/// `fixtures::archive(true)` with one more entry, `name`.
fn archive_with(name: &str, bytes: &[u8]) -> Vec<u8> {
    let mut source = zip::ZipArchive::new(std::io::Cursor::new(fixtures::archive(true))).unwrap();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for index in 0..source.len() {
        let mut entry = source.by_index(index).unwrap();
        let entry_name = entry.name().unwrap().into_owned();
        out.start_file(entry_name, zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::copy(&mut entry, &mut out).unwrap();
    }
    out.start_file(name, zip::write::SimpleFileOptions::default())
        .unwrap();
    out.write_all(bytes).unwrap();
    out.finish().unwrap().into_inner()
}

/// Fails at `Point::Replaced`: the modified original is set aside, and the
/// replacement is not copied yet.
fn crash_at_replaced(point: publication::Point) -> Result<(), Error> {
    if point == publication::Point::Replaced {
        Err(StorageError::Io.into())
    } else {
        Ok(())
    }
}

/// The work folder and the backup folder, by name, left in the folder's root.
fn leftover_names(f: &Fixture) -> usize {
    std::fs::read_dir(f.folder())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.starts_with(".cimmeria-adopt-") || name.starts_with("cimmeria-adoption-backup-")
        })
        .count()
}

/// A folder whose SGW.exe the player edited, with the release bytes stored as its
/// replacement. Returns the release bytes too. `later.txt` stays: the fixture's
/// release has no other file, and a folder with nothing reusable is refused.
fn modified_with_replacement() -> (Fixture, Preview, Vec<u8>) {
    let f = Fixture::new(None);
    let exe_release = std::fs::read(inventory::native(f.folder(), EXE)).unwrap();
    std::fs::write(inventory::native(f.folder(), EXE), EDITED).unwrap();
    let mut preview = f.preview().unwrap();
    let exe = stored_replacement(&preview, EXE, &exe_release);
    preview.replacements = vec![exe];
    (f, preview, exe_release)
}

#[test]
fn adopting_with_consent_backs_up_the_modified_file_and_installs_the_release() {
    let (f, preview, exe_release) = modified_with_replacement();
    let backup = preview.report.backup.clone().unwrap();
    assert!(backup.file_name().is_some_and(|name| name
        .to_string_lossy()
        .starts_with("cimmeria-adoption-backup-")));
    let expected = preview.expected_digest;
    let provenance = confirm_replacing(preview, |_| Ok(())).unwrap();

    // The player's edited SGW.exe is kept byte for byte in the backup folder.
    assert_eq!(
        std::fs::read(inventory::native(&backup, EXE)).unwrap(),
        EDITED
    );
    // The work folder was renamed away by the publication.
    let leftovers = std::fs::read_dir(f.folder())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".cimmeria-adopt-"))
        .count();
    assert_eq!(leftovers, 0);
    // The game files hold the release bytes.
    let game = f.folder().to_path_buf();
    assert_eq!(
        std::fs::read(inventory::native(&game, EXE)).unwrap(),
        exe_release
    );
    assert_eq!(
        inventory::content_digest(
            &super::direct::scan(
                &game,
                &inspect(&f.state.lock().unwrap(), provenance.work_id)
                    .unwrap()
                    .plan
            )
            .unwrap()
        )
        .unwrap(),
        expected
    );
}

#[test]
fn uninstalling_an_adoption_that_replaced_files_keeps_the_backup() {
    let (f, preview, _) = modified_with_replacement();
    let backup = preview.report.backup.clone().unwrap();
    let _provenance = confirm_replacing(preview, |_| Ok(())).unwrap();
    let mut state = f.state.lock().unwrap();
    let installed = state.installed_content().unwrap().unwrap();
    let revision = state.operations().snapshot().revision;
    state
        .uninstall(
            Uuid::new_v4(),
            revision,
            installed.intent.operation_id,
            true,
        )
        .unwrap();
    drop(state);
    assert!(backup.is_dir());
    assert_eq!(
        std::fs::read(inventory::native(&backup, EXE)).unwrap(),
        EDITED
    );
}

#[test]
fn a_missing_release_file_is_restored_from_its_replacement_and_the_others_are_reused() {
    let f = Fixture::with_archive(None, SeedKind::Zip, three_file_archive());
    std::fs::remove_file(f.folder().join("extra.txt")).unwrap();
    let mut preview = f.preview().unwrap();
    let mut extra = stored_replacement(&preview, "extra.txt", EXTRA);
    extra.reason = Classification::Missing;
    preview.replacements = vec![extra];
    let _provenance = confirm_replacing(preview, |_| Ok(())).unwrap();

    // The missing file comes from its replacement; the release files still there
    // are moved into `game` as before.
    let game = f.folder().to_path_buf();
    assert_eq!(std::fs::read(game.join("extra.txt")).unwrap(), EXTRA);
    assert_eq!(
        std::fs::read(game.join("later.txt")).unwrap(),
        b"later entry"
    );
    assert!(f.folder().join("extra.txt").exists());
}

#[test]
fn a_crash_at_replaced_is_finished_by_recovery_with_the_release_in_game() {
    let (f, preview, exe_release) = modified_with_replacement();
    let backup = preview.report.backup.clone().unwrap();
    assert!(confirm_replacing(preview, crash_at_replaced).is_err());
    let f = f.reopen();
    assert_eq!(
        std::fs::read(inventory::native(&backup, EXE)).unwrap(),
        EDITED
    );

    let (id, _) = f.operation();
    let mut state = f.state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    recover(&mut state, id, revision).unwrap();
    drop(state);

    let game = f.folder().to_path_buf();
    assert_eq!(
        std::fs::read(inventory::native(&game, EXE)).unwrap(),
        exe_release
    );
    assert_eq!(
        std::fs::read(inventory::native(&backup, EXE)).unwrap(),
        EDITED
    );
}

#[test]
fn a_crash_at_replaced_abandoned_puts_the_modified_original_back_byte_for_byte() {
    let (f, preview, _) = modified_with_replacement();
    let backup = preview.report.backup.clone().unwrap();
    assert!(confirm_replacing(preview, crash_at_replaced).is_err());
    let f = f.reopen();

    let (id, _) = f.operation();
    let mut state = f.state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    abandon(&mut state, id, revision).unwrap();
    drop(state);

    assert_eq!(
        std::fs::read(inventory::native(f.folder(), EXE)).unwrap(),
        EDITED
    );
    assert!(!backup.exists());
    assert_eq!(leftover_names(&f), 0);
}

/// The seed's file the folder spells in another case.
const CASED: &str = "Working/Binaries/tool.bin";
const CASED_BYTES: &[u8] = b"cased tool";
/// Added by the overlay patch, so the folder has no copy of it.
const PATCH_ADDED: &str = "Working/Binaries/added.txt";
const PATCH_ADDED_BYTES: &[u8] = b"added by patch";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A release signed with the test key, whose one overlay patch is `overlay`.
fn signed_with_overlay(seed: &[u8], overlay: &[u8]) -> VerifiedRelease {
    let body = serde_json::to_vec(&serde_json::json!({
        "schema": 1,
        "seed": {"blob": "seed.zip", "size": seed.len(), "sha256": hex(&Sha256::digest(seed))},
        "patches": [{
            "id": "overlay",
            "blob": "overlay.zip",
            "size": overlay.len(),
            "sha256": hex(&Sha256::digest(overlay)),
            "root": "install_dir",
        }],
    }))
    .unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    crate::catalog::verify_release(&body, hex(&signature.to_bytes()).as_bytes()).unwrap()
}

/// An overlay that adds `PATCH_ADDED` and nothing else.
fn overlay_archive() -> Vec<u8> {
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    out.start_file(PATCH_ADDED, zip::write::SimpleFileOptions::default())
        .unwrap();
    out.write_all(PATCH_ADDED_BYTES).unwrap();
    out.finish().unwrap().into_inner()
}

/// The folder's `Working/Binaries` is spelled `Working/binaries`, and the release
/// has a file the overlay adds. Admission must not fail on either, and publishing
/// puts the release bytes in `game`, the patch-added file included.
#[cfg(windows)]
#[test]
fn a_patch_added_file_and_a_folder_spelled_in_another_case_are_adopted() {
    let seed = archive_with(CASED, CASED_BYTES);
    let overlay = overlay_archive();
    let dir = tempfile::tempdir().unwrap();
    let overlay_path = dir.path().join("overlay.zip");
    std::fs::write(&overlay_path, &overlay).unwrap();
    let mut f = Fixture::with_archive(None, SeedKind::Zip, seed.clone());
    f.release = signed_with_overlay(&seed, &overlay);
    let binaries = inventory::native(f.folder(), "Working/Binaries");
    let renamed = inventory::native(f.folder(), "Working/binaries-renamed");
    std::fs::rename(&binaries, &renamed).unwrap();
    std::fs::rename(&renamed, inventory::native(f.folder(), "Working/binaries")).unwrap();
    let mut request = f.request();
    request
        .artifacts
        .as_mut()
        .unwrap()
        .patches
        .push(overlay_path);
    let preview = preview(
        f.state.clone(),
        request,
        CancellationToken::new(),
        crate::install_progress::ProgressSink::latest().0,
    )
    .unwrap();
    let expected = preview.expected_digest;
    let provenance = commit(preview).unwrap();

    let game = f.folder().to_path_buf();
    assert_eq!(
        std::fs::read(inventory::native(&game, PATCH_ADDED)).unwrap(),
        PATCH_ADDED_BYTES
    );
    assert_eq!(
        std::fs::read(inventory::native(&game, CASED)).unwrap(),
        CASED_BYTES
    );
    assert_eq!(
        inventory::content_digest(
            &super::direct::scan(
                &game,
                &inspect(&f.state.lock().unwrap(), provenance.work_id)
                    .unwrap()
                    .plan
            )
            .unwrap()
        )
        .unwrap(),
        expected
    );
}

/// The login file the setup rule writes, relative to the folder.
const LOGIN: &str = "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua";
const CUSTOM_LOGIN: &[u8] = b"-- the player's own login servers\n";

/// A release that names `LoginInternal.lua`, and a folder whose copy the player
/// wrote. The custom file is a known transform: it adopts in place with no
/// replacement, and its original goes to the backup at the same path.
#[test]
fn a_custom_login_file_adopts_in_place_and_its_original_is_backed_up() {
    let f = Fixture::with_archive(None, SeedKind::Zip, archive_with(LOGIN, b"stock login"));
    let login = inventory::native(f.folder(), LOGIN);
    std::fs::create_dir_all(login.parent().unwrap()).unwrap();
    std::fs::write(&login, CUSTOM_LOGIN).unwrap();
    let preview = f.preview().unwrap();
    assert!(preview.report.login_replaced);
    let backup = preview.report.backup.clone().unwrap();
    let _provenance = commit(preview).unwrap();

    // The player's login file is kept at the same relative path in the backup.
    assert_eq!(
        std::fs::read(inventory::native(&backup, LOGIN)).unwrap(),
        CUSTOM_LOGIN
    );
    // The game's login file is the setup version, not the player's.
    let game_login = std::fs::read(inventory::native(f.folder(), LOGIN)).unwrap();
    assert_ne!(game_login, CUSTOM_LOGIN);
}

/// A launcher-owned work folder that holds a file no adoption writes is refused,
/// and the file is kept.
#[test]
fn a_work_folder_with_an_unexpected_file_is_refused_and_the_file_is_kept() {
    let f = Fixture::new(None);
    let stage = f
        .folder()
        .join(format!(".cimmeria-adopt-{}", Uuid::new_v4()));
    std::fs::create_dir(&stage).unwrap();
    std::fs::write(stage.join("stray.txt"), b"unexpected").unwrap();
    assert_eq!(f.preview().err(), Some(Error::LeftoverLauncherFiles));
    assert_eq!(
        std::fs::read(stage.join("stray.txt")).unwrap(),
        b"unexpected"
    );
}

/// `Working` is a junction to another folder. The in-place preview must refuse
/// before it writes its write probe, so the junction target gets no probe file.
#[cfg(windows)]
#[test]
fn an_in_place_preview_writes_no_probe_through_a_junction() {
    let f = Fixture::new(None);
    let target = tempfile::tempdir().unwrap();
    let working = f.folder().join("Working");
    std::fs::remove_dir_all(&working).unwrap();
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&working)
        .arg(target.path())
        .status();
    if !matches!(made, Ok(status) if status.success()) {
        return;
    }
    assert!(f.preview().is_err());
    let probes = std::fs::read_dir(target.path())
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".cimmeria-write-probe-")
        })
        .count();
    assert_eq!(probes, 0);
}

/// Uninstall removes a `.cimmeria-adopt-<work id>` folder that an adoption left
/// behind, and the backup folder stays.
#[test]
fn uninstalling_removes_a_work_folder_an_adoption_left_behind() {
    let (f, preview, _) = modified_with_replacement();
    let backup = preview.report.backup.clone().unwrap();
    let provenance = confirm_replacing(preview, |_| Ok(())).unwrap();
    let stuck = f
        .folder()
        .join(format!(".cimmeria-adopt-{}", provenance.work_id));
    std::fs::create_dir(&stuck).unwrap();
    std::fs::write(stuck.join("left.txt"), b"left behind").unwrap();
    let mut state = f.state.lock().unwrap();
    let installed = state.installed_content().unwrap().unwrap();
    let revision = state.operations().snapshot().revision;
    state
        .uninstall(
            Uuid::new_v4(),
            revision,
            installed.intent.operation_id,
            true,
        )
        .unwrap();
    drop(state);
    assert!(!stuck.exists());
    assert!(backup.is_dir());
}

/// Abandon removes a `<game file>.adopt-partial` left by an interrupted copy, and
/// no game folder is left behind.
#[test]
fn abandoning_removes_a_partial_replacement_left_in_game() {
    let (f, preview, _) = modified_with_replacement();
    assert!(confirm_replacing(preview, crash_at_replaced).is_err());
    let f = f.reopen();
    let mut partial = inventory::native(f.folder(), EXE).into_os_string();
    partial.push(".adopt-partial");
    let partial = PathBuf::from(partial);
    std::fs::create_dir_all(partial.parent().unwrap()).unwrap();
    std::fs::write(&partial, b"half copied").unwrap();

    let (id, _) = f.operation();
    let mut state = f.state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    abandon(&mut state, id, revision).unwrap();
    drop(state);

    assert!(!partial.exists());
    assert!(!f.folder().join("game").exists());
    assert_eq!(
        std::fs::read(inventory::native(f.folder(), EXE)).unwrap(),
        EDITED
    );
}
