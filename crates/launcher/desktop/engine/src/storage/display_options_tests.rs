use super::*;
use crate::OperationKind;
use uuid::Uuid;

const OLD_FILE: &str = r#"{"schema_version":2,"revision":3,"install_directory":null,"launcher_summary_consent":false,"login_servers":[],"client_patches_enabled":true}"#;

fn sized(mode: DisplayMode, width: u32, height: u32) -> DisplayOptions {
    DisplayOptions {
        mode,
        width,
        height,
    }
}

#[test]
fn the_default_is_a_1024x768_window_on_every_platform() {
    assert_eq!(
        DisplayOptions::default(),
        sized(DisplayMode::Windowed, 1024, 768)
    );
    assert!(DisplayOptions::default().valid());
}

#[test]
fn a_preferences_file_from_before_the_display_option_reads_the_default() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("preferences.json"), OLD_FILE).unwrap();
    let state = DesktopState::open(dir.path()).unwrap();
    assert_eq!(state.preferences().revision, 3);
    assert_eq!(state.preferences().display, DisplayOptions::default());
}

#[test]
fn an_unknown_stored_mode_is_corrupt_like_any_other_bad_preference() {
    // "game" (a "keep the game's own setting" mode) never shipped. A file that
    // names it fails to parse, and opening reports Corrupt rather than
    // silently resetting the file: the same path as every corrupt value.
    for display in [
        r#"{"mode":"game","width":1024,"height":768}"#,
        r#"{"mode":"windowed","width":1024,"height":768,"extra":1}"#,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let file = OLD_FILE.replace('}', &format!(r#","display":{display}}}"#));
        std::fs::write(dir.path().join("preferences.json"), &file).unwrap();
        assert!(
            matches!(DesktopState::open(dir.path()), Err(StorageError::Corrupt)),
            "{display}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("preferences.json")).unwrap(),
            file
        );
    }
}

#[test]
fn a_saved_display_survives_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let chosen = sized(DisplayMode::Windowed, 1600, 900);
    {
        let mut state = DesktopState::open(dir.path()).unwrap();
        let saved = state.save_display_options(chosen, 0).unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(saved.display, chosen);
        // The other preferences are untouched.
        assert!(saved.client_patches_enabled && saved.login_servers.is_empty());
    }
    let reopened = DesktopState::open(dir.path()).unwrap();
    assert_eq!(reopened.preferences().display, chosen);
    assert_eq!(reopened.preferences().revision, 1);
}

#[test]
fn an_out_of_range_size_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    for invalid in [
        sized(DisplayMode::Windowed, 320, 768),
        sized(DisplayMode::Windowed, 1024, 200),
        sized(DisplayMode::Windowed, 8000, 768),
    ] {
        assert_eq!(
            state.save_display_options(invalid, 0),
            Err(StorageError::InvalidDisplayOptions),
            "{invalid:?}"
        );
        assert_eq!(state.preferences(), &Preferences::default());
        assert!(!dir.path().join("preferences.json").exists());
    }
}

#[test]
fn a_stale_revision_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    assert_eq!(
        state.save_display_options(sized(DisplayMode::Windowed, 1600, 900), 4),
        Err(StorageError::StaleRevision)
    );
    assert!(!dir.path().join("preferences.json").exists());
}

#[test]
fn an_open_install_refuses_the_save() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    state
        .operations_mut()
        .unwrap()
        .begin(Uuid::new_v4(), OperationKind::Install, [1; 32], 0)
        .unwrap();
    assert_eq!(
        state.save_display_options(sized(DisplayMode::Windowed, 1600, 900), 0),
        Err(StorageError::Busy)
    );
    assert_eq!(state.preferences(), &Preferences::default());
}

#[test]
fn effective_clamps_the_size() {
    assert_eq!(
        sized(DisplayMode::Windowed, 100, 9000).effective(),
        sized(DisplayMode::Windowed, 640, 4320)
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_offers_only_a_window_and_never_applies_fullscreen() {
    assert_eq!(allowed_modes(), &[DisplayMode::Windowed]);
    let fullscreen = sized(DisplayMode::Fullscreen, 1920, 1080);
    assert!(!fullscreen.valid());
    assert_eq!(
        fullscreen.effective(),
        sized(DisplayMode::Windowed, 1920, 1080)
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn other_platforms_offer_and_apply_fullscreen() {
    assert_eq!(
        allowed_modes(),
        &[DisplayMode::Windowed, DisplayMode::Fullscreen]
    );
    let fullscreen = sized(DisplayMode::Fullscreen, 1920, 1080);
    assert!(fullscreen.valid());
    assert_eq!(fullscreen.effective(), fullscreen);
}

#[test]
fn a_default_display_is_stored_as_no_key_and_round_trips() {
    let default = Preferences::default();
    assert_eq!(
        serde_json::to_string(&default).unwrap(),
        r#"{"schema_version":2,"revision":0,"install_directory":null,"launcher_summary_consent":false,"login_servers":[],"client_patches_enabled":true}"#
    );
    let back: Preferences =
        serde_json::from_str(&serde_json::to_string(&default).unwrap()).unwrap();
    assert_eq!(back, default);
    let custom = Preferences {
        display: sized(DisplayMode::Windowed, 1600, 900),
        ..Preferences::default()
    };
    let text = serde_json::to_string(&custom).unwrap();
    assert!(text.contains(r#""display":{"mode":"windowed","width":1600,"height":900}"#));
    assert_eq!(serde_json::from_str::<Preferences>(&text).unwrap(), custom);
}

#[test]
fn the_display_is_covered_by_the_record_digest() {
    let default = Preferences::default();
    let custom = Preferences {
        display: sized(DisplayMode::Windowed, 1600, 900),
        ..Preferences::default()
    };
    let a = serde_json::to_string(&default.digest_form()).unwrap();
    let b = serde_json::to_string(&custom.digest_form()).unwrap();
    assert_ne!(a, b);
    assert!(!a.contains("display"));
    assert!(a.contains(r#""schema_version":1"#));
    assert!(b.contains("display"));
}
