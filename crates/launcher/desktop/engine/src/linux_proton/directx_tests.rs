use super::*;
use std::cell::Cell;

/// Fixture bytes that stand in for a DLL, with a pin for them.
const GOOD: &[u8] = b"native d3dx9_40 fixture";

fn pin(dll: &'static str, bytes: &[u8]) -> Pinned {
    Pinned {
        cab: "Fixture_x86.cab",
        dll,
        bytes: bytes.len() as u64,
        sha256: Box::leak(sha256_hex(bytes).into_boxed_str()),
    }
}

/// A claimed prefix under `root` with its `syswow64` folder.
fn prefix_with_syswow64(root: &Path) -> Prefix {
    let prefix = super::super::prefix::claim(root).unwrap();
    fs::create_dir_all(prefix.pfx.join(SYSWOW64)).unwrap();
    prefix
}

/// A game root whose prerequisite folder holds the fixture cabinet, spelled
/// as `folder`.
fn game_with_cab(root: &Path, folder: &str) -> std::path::PathBuf {
    let game = root.join("game");
    fs::create_dir_all(game.join(".cimmeria-prerequisites").join(folder)).unwrap();
    fs::write(
        game.join(".cimmeria-prerequisites")
            .join(folder)
            .join("Fixture_x86.cab"),
        b"cab",
    )
    .unwrap();
    game
}

/// The extractor the tests use: it returns `bytes` when the cabinet exists and
/// counts its calls.
fn fake<'a>(
    bytes: &'a [u8],
    calls: &'a Cell<u32>,
) -> impl Fn(&Path, &str, u64) -> Result<Vec<u8>, UnpackError> + 'a {
    move |cab, _member, _max| {
        calls.set(calls.get() + 1);
        if !cab.is_file() {
            return Err(UnpackError::Cab("no such cabinet".into()));
        }
        Ok(bytes.to_vec())
    }
}

fn target_path(prefix: &Prefix, dll: &str) -> std::path::PathBuf {
    prefix.pfx.join(SYSWOW64).join(dll)
}

#[test]
fn missing_target_is_extracted_written_and_matches_its_pin() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_with_syswow64(root.path());
    let game = game_with_cab(root.path(), "DX9.0c");
    let pins = [pin("d3dx9_40.dll", GOOD)];
    let calls = Cell::new(0);
    ensure_with(&prefix, &game, &pins, fake(GOOD, &calls)).unwrap();
    assert_eq!(calls.get(), 1);
    let target = target_path(&prefix, "d3dx9_40.dll");
    assert_eq!(fs::read(&target).unwrap(), GOOD);
    assert!(!prefix
        .pfx
        .join(SYSWOW64)
        .join("d3dx9_40.dll.cimmeria-tmp")
        .exists());
}

#[test]
fn matching_target_is_kept_and_the_extractor_is_not_called() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_with_syswow64(root.path());
    let game = game_with_cab(root.path(), "DX9.0c");
    let pins = [pin("d3dx9_40.dll", GOOD)];
    fs::write(target_path(&prefix, "d3dx9_40.dll"), GOOD).unwrap();
    let calls = Cell::new(0);
    ensure_with(&prefix, &game, &pins, fake(GOOD, &calls)).unwrap();
    assert_eq!(calls.get(), 0);
}

#[test]
fn wrong_target_is_replaced() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_with_syswow64(root.path());
    let game = game_with_cab(root.path(), "DX9.0c");
    let pins = [pin("d3dx9_40.dll", GOOD)];
    // Same length as GOOD, different bytes: only the hash can tell them apart.
    let stale = vec![b'x'; GOOD.len()];
    fs::write(target_path(&prefix, "d3dx9_40.dll"), &stale).unwrap();
    let calls = Cell::new(0);
    ensure_with(&prefix, &game, &pins, fake(GOOD, &calls)).unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        fs::read(target_path(&prefix, "d3dx9_40.dll")).unwrap(),
        GOOD
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_at_the_target_is_replaced_by_a_regular_file() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_with_syswow64(root.path());
    let game = game_with_cab(root.path(), "DX9.0c");
    let pins = [pin("d3dx9_40.dll", GOOD)];
    let elsewhere = root.path().join("elsewhere.dll");
    fs::write(&elsewhere, GOOD).unwrap();
    std::os::unix::fs::symlink(&elsewhere, target_path(&prefix, "d3dx9_40.dll")).unwrap();
    let calls = Cell::new(0);
    ensure_with(&prefix, &game, &pins, fake(GOOD, &calls)).unwrap();
    // The link is not a regular file, so it is extracted again and replaced.
    assert_eq!(calls.get(), 1);
    let meta = fs::symlink_metadata(target_path(&prefix, "d3dx9_40.dll")).unwrap();
    assert!(meta.is_file());
}

#[test]
fn wrong_extracted_bytes_are_refused_and_leave_the_old_target_untouched() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_with_syswow64(root.path());
    let game = game_with_cab(root.path(), "DX9.0c");
    let pins = [pin("d3dx9_40.dll", GOOD)];
    let old = b"old dll".to_vec();
    fs::write(target_path(&prefix, "d3dx9_40.dll"), &old).unwrap();
    let calls = Cell::new(0);
    let wrong = b"not the pinned dll";
    let err = ensure_with(&prefix, &game, &pins, fake(wrong, &calls)).unwrap_err();
    assert!(matches!(err, ProtonError::Setup("directx")));
    assert_eq!(fs::read(target_path(&prefix, "d3dx9_40.dll")).unwrap(), old);
    assert!(!prefix
        .pfx
        .join(SYSWOW64)
        .join("d3dx9_40.dll.cimmeria-tmp")
        .exists());
}

#[test]
fn extracted_bytes_of_the_right_length_but_the_wrong_hash_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_with_syswow64(root.path());
    let game = game_with_cab(root.path(), "DX9.0c");
    let pins = [pin("d3dx9_40.dll", GOOD)];
    let old = b"old dll".to_vec();
    fs::write(target_path(&prefix, "d3dx9_40.dll"), &old).unwrap();
    let calls = Cell::new(0);
    // Same length as the pin, different bytes: only the hash can refuse them.
    let wrong = vec![b'y'; GOOD.len()];
    let err = ensure_with(&prefix, &game, &pins, fake(&wrong, &calls)).unwrap_err();
    assert!(matches!(err, ProtonError::Setup("directx")));
    assert_eq!(fs::read(target_path(&prefix, "d3dx9_40.dll")).unwrap(), old);
    assert!(!prefix
        .pfx
        .join(SYSWOW64)
        .join("d3dx9_40.dll.cimmeria-tmp")
        .exists());
}

#[cfg(unix)]
#[test]
fn a_symlink_at_the_temporary_file_is_not_followed() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_with_syswow64(root.path());
    let game = game_with_cab(root.path(), "DX9.0c");
    let pins = [pin("d3dx9_40.dll", GOOD)];
    let outside = root.path().join("outside.dll");
    fs::write(&outside, b"outside").unwrap();
    let temp = prefix.pfx.join(SYSWOW64).join("d3dx9_40.dll.cimmeria-tmp");
    std::os::unix::fs::symlink(&outside, &temp).unwrap();
    let calls = Cell::new(0);
    ensure_with(&prefix, &game, &pins, fake(GOOD, &calls)).unwrap();
    assert_eq!(fs::read(&outside).unwrap(), b"outside");
    let meta = fs::symlink_metadata(target_path(&prefix, "d3dx9_40.dll")).unwrap();
    assert!(meta.is_file());
    assert_eq!(
        fs::read(target_path(&prefix, "d3dx9_40.dll")).unwrap(),
        GOOD
    );
}

#[test]
fn a_lower_case_prerequisite_folder_on_disk_is_still_found() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_with_syswow64(root.path());
    // The fake extractor refuses a missing cabinet, so a literal `DX9.0c` join
    // (which does not exist here) would fail this test.
    let game = game_with_cab(root.path(), "dx9.0c");
    let pins = [pin("d3dx9_40.dll", GOOD)];
    let calls = Cell::new(0);
    ensure_with(&prefix, &game, &pins, fake(GOOD, &calls)).unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        fs::read(target_path(&prefix, "d3dx9_40.dll")).unwrap(),
        GOOD
    );
}

#[test]
fn a_prefix_without_syswow64_is_a_setup_failure() {
    let root = tempfile::tempdir().unwrap();
    let prefix = super::super::prefix::claim(root.path()).unwrap();
    let game = game_with_cab(root.path(), "DX9.0c");
    let pins = [pin("d3dx9_40.dll", GOOD)];
    let calls = Cell::new(0);
    let err = ensure_with(&prefix, &game, &pins, fake(GOOD, &calls)).unwrap_err();
    assert!(matches!(err, ProtonError::Setup("directx")));
    assert_eq!(calls.get(), 0);
}

#[test]
fn a_missing_cabinet_is_a_setup_failure() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_with_syswow64(root.path());
    let game = root.path().join("game");
    fs::create_dir_all(&game).unwrap();
    let pins = [pin("d3dx9_40.dll", GOOD)];
    let calls = Cell::new(0);
    let err = ensure_with(&prefix, &game, &pins, fake(GOOD, &calls)).unwrap_err();
    assert!(matches!(err, ProtonError::Setup("directx")));
    assert!(!target_path(&prefix, "d3dx9_40.dll").exists());
}

#[test]
fn the_production_pins_name_both_dlls_and_their_overrides() {
    assert_eq!(DLLS[0].dll, "d3dx9_40.dll");
    assert_eq!(DLLS[1].dll, "xinput1_3.dll");
    assert_eq!(OVERRIDES, "d3dx9_40=n;xinput1_3=n");
    assert_eq!(DLLS[0].cab, "Nov2008_d3dx9_40_x86.cab");
    assert_eq!(DLLS[0].bytes, 4_379_984);
    assert_eq!(
        DLLS[0].sha256,
        "16fd909aeb68d0d1aca8529dc7f78880b97d6649d70ce8d03a2c858bc28e216b"
    );
    assert_eq!(DLLS[1].cab, "APR2007_xinput_x86.cab");
    assert_eq!(DLLS[1].bytes, 81_768);
    assert_eq!(
        DLLS[1].sha256,
        "8d540d484ea41e374fd0107d55d253f87ded4ce780d515d8fd59bbe8c98970a7"
    );
}
