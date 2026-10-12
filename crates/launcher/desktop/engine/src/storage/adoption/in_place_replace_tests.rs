//! In-place adoption that replaces a modified release file with consent, and
//! keeps the player's original in the backup folder (AI-11).
use super::in_place_tests::*;
use super::*;

const EXE: &str = "Working/Binaries/SGW.exe";
const EDITED: &[u8] = b"patched by the player";

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
    let expected = inventory::content_digest(&preview.reference.index).unwrap();
    confirm_replacing(preview, |_| Ok(())).unwrap();

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
    let game = f.folder().join("game");
    assert_eq!(
        std::fs::read(inventory::native(&game, EXE)).unwrap(),
        exe_release
    );
    assert_eq!(
        inventory::content_digest(&inventory::scan(&game, &CancellationToken::new()).unwrap())
            .unwrap(),
        expected
    );
}

#[test]
fn uninstalling_an_adoption_that_replaced_files_keeps_the_backup() {
    let (f, preview, _) = modified_with_replacement();
    let backup = preview.report.backup.clone().unwrap();
    confirm_replacing(preview, |_| Ok(())).unwrap();
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
