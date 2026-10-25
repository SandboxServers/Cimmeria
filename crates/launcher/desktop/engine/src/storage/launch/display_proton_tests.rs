//! The Proton display step (PP-17) on temp prefixes: the profile path, the first
//! Play's seed, the symlink refusal and the edit of an existing file.
use super::*;
use crate::test_capture::{capture, Captured};
use tracing::Level;

/// Two options, one of them `windowedMode` false: the seed would drop `resolution`.
const SAVED_WINDOWED_OFF: &str = concat!(
    "<options valueInt=\"3\" version=\"1\" optionName=\"resolution\" groupName=\"video\"></options>\r\n",
    "<options valueBool=\"false\" version=\"1\" optionName=\"windowedMode\" groupName=\"video\"></options>\r\n",
);

/// The steamuser folder of a fake prefix.
fn steamuser(pfx: &Path) -> PathBuf {
    pfx.join("drive_c/users/steamuser")
}

/// The Proton client profile in a fake prefix.
fn profile_in(pfx: &Path) -> PathBuf {
    steamuser(pfx).join("Documents/My Games/Firesky/SGWGame")
}

fn applied(pfx: &Path, profile: &Path, display: DisplayOptions) -> Captured {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let ((), events) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        apply_proton(Uuid::new_v4(), pfx, profile, display)
    });
    let mut matching = events
        .into_iter()
        .filter(|e| e.field("event") == Some("display_settings"));
    let event = matching.next().expect("one display_settings event");
    assert!(matching.next().is_none());
    assert_eq!(event.target, "launcher.launch");
    event
}

#[test]
fn proton_profile_is_the_documents_game_folder_of_the_steamuser_profile() {
    let state = tempfile::tempdir().unwrap();
    let prefix = crate::linux_proton::prefix::claim(state.path()).unwrap();
    assert_eq!(
        proton_profile(&prefix),
        prefix
            .pfx
            .join("drive_c/users/steamuser/Documents/My Games/Firesky/SGWGame")
    );
}

#[test]
fn the_seed_is_exactly_441_bytes_and_already_windowed() {
    assert_eq!(SEED_SAVED_OPTIONS.len(), 441);
    // Later Plays edit the seed, so windowed=true must hand it back unchanged.
    assert_eq!(
        saved_options(SEED_SAVED_OPTIONS, true).as_deref(),
        Some(SEED_SAVED_OPTIONS)
    );
}

#[test]
fn a_first_play_seeds_the_saved_options_and_never_creates_the_engine_ini() {
    let pfx = tempfile::tempdir().unwrap();
    let profile = profile_in(pfx.path());
    std::fs::create_dir_all(steamuser(pfx.path()).join("Documents")).unwrap();
    let event = applied(pfx.path(), &profile, DisplayOptions::default());
    assert_eq!(event.level, Level::WARN);
    assert_eq!(event.field("mode"), Some("windowed"));
    assert_eq!(event.field("saved_options"), Some("seeded"));
    assert_eq!(event.field("engine_ini"), Some("missing"));
    let written = std::fs::read(profile.join(SAVED_OPTIONS)).unwrap();
    assert_eq!(written, SEED_SAVED_OPTIONS.as_bytes());
    assert!(!profile.join("Config").exists());
}

#[test]
fn an_existing_saved_options_file_is_edited_and_not_replaced_by_the_seed() {
    let pfx = tempfile::tempdir().unwrap();
    let profile = profile_in(pfx.path());
    std::fs::create_dir_all(&profile).unwrap();
    std::fs::write(profile.join(SAVED_OPTIONS), SAVED_WINDOWED_OFF).unwrap();
    let event = applied(pfx.path(), &profile, DisplayOptions::default());
    assert_eq!(event.field("saved_options"), Some("written"));
    let saved = std::fs::read_to_string(profile.join(SAVED_OPTIONS)).unwrap();
    assert_eq!(saved, saved_options(SAVED_WINDOWED_OFF, true).unwrap());
    assert!(saved.contains("resolution"), "{saved}");
}

#[cfg(unix)]
#[test]
fn a_documents_folder_linked_out_of_the_prefix_writes_nothing() {
    let pfx = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(steamuser(pfx.path())).unwrap();
    std::os::unix::fs::symlink(outside.path(), steamuser(pfx.path()).join("Documents")).unwrap();
    let event = applied(
        pfx.path(),
        &profile_in(pfx.path()),
        DisplayOptions::default(),
    );
    assert_eq!(event.field("saved_options"), Some("failed"));
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn a_first_play_with_an_existing_engine_ini_is_reported_as_applied() {
    let pfx = tempfile::tempdir().unwrap();
    let profile = profile_in(pfx.path());
    std::fs::create_dir_all(profile.join("Config")).unwrap();
    std::fs::write(
        profile.join(ENGINE_INI),
        "[WinDrv.WindowsClient]\r\nStartupFullscreen=True\r\nStartupResolutionX=800\r\nStartupResolutionY=600\r\n",
    )
    .unwrap();
    let event = applied(pfx.path(), &profile, DisplayOptions::default());
    assert_eq!(event.field("saved_options"), Some("seeded"));
    assert_eq!(event.field("engine_ini"), Some("written"));
    assert_eq!(event.level, Level::INFO);
}
