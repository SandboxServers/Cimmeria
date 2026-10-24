use super::*;
use crate::display_targets::setup_key;
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
        let saved = state.save_display_options(chosen, None, 0).unwrap();
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
            state.save_display_options(invalid, None, 0),
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
        state.save_display_options(sized(DisplayMode::Windowed, 1600, 900), None, 4),
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
        state.save_display_options(sized(DisplayMode::Windowed, 1600, 900), None, 0),
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

// C-PD8 placements (PD-32).

fn display(index: u32, x: i32, width: u32, height: u32) -> DisplayInfo {
    DisplayInfo {
        index,
        main: index == 0,
        x,
        y: 0,
        width,
        height,
        scale_percent: 100,
        refresh_hz: 60,
    }
}

fn rect(d: &DisplayInfo) -> MonitorRect {
    MonitorRect {
        x: d.x,
        y: d.y,
        width: d.width,
        height: d.height,
    }
}

fn at(x: i32) -> MonitorRect {
    MonitorRect {
        x,
        y: 0,
        width: 1920,
        height: 1080,
    }
}

#[test]
fn remember_replaces_removes_and_keeps_the_newest_eight() {
    let mut list = Vec::new();
    remember(&mut list, "a", Some(at(0)));
    remember(&mut list, "b", Some(at(1)));
    remember(&mut list, "a", Some(at(2)));
    // One entry per key; the replacement is now the newest.
    assert_eq!(list.len(), 2);
    assert_eq!((list[1].setup.as_str(), list[1].x), ("a", 2));
    remember(&mut list, "a", None);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].setup, "b");
    remember(&mut list, "absent", None);
    assert_eq!(list.len(), 1);
    for i in 0..10 {
        remember(&mut list, &format!("k{i}"), Some(at(i)));
    }
    assert_eq!(list.len(), MAX_PLACEMENTS);
    let keys: Vec<&str> = list.iter().map(|p| p.setup.as_str()).collect();
    assert_eq!(keys, ["k2", "k3", "k4", "k5", "k6", "k7", "k8", "k9"]);
}

#[test]
fn placement_for_needs_the_setup_key_and_a_matching_display() {
    let main = display(0, 0, 1710, 1107);
    let side = display(1, 1710, 1920, 1080);
    let setup = [main, side];
    let mut list = Vec::new();
    remember(&mut list, &setup_key(&setup), Some(rect(&side)));
    assert_eq!(placement_for(&list, &setup), Some(side));
    // Revert proof: another setup that holds the same rectangle is unknown, so
    // it gives None; matching on the rectangle alone would return `side`.
    let other = [main, side, display(2, -1920, 1920, 1080)];
    assert_eq!(placement_for(&list, &other), None);
    // No displays, and a stored rectangle no display has, give None.
    assert_eq!(placement_for(&list, &[]), None);
    let mut moved = Vec::new();
    remember(&mut moved, &setup_key(&setup), Some(at(9999)));
    assert_eq!(placement_for(&moved, &setup), None);
}

#[test]
fn a_saved_monitor_is_found_again_after_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let setup = [display(0, 0, 1710, 1107), display(1, 1710, 1920, 1080)];
    {
        let mut state = DesktopState::open(dir.path()).unwrap();
        let (saved, remembered) = state
            .save_display_options_on(DisplayOptions::default(), Some(rect(&setup[1])), 0, &setup)
            .unwrap();
        assert!(remembered);
        assert_eq!(saved.display_placements.len(), 1);
        assert_eq!(saved.display_placements[0].setup, setup_key(&setup));
    }
    let mut reopened = DesktopState::open(dir.path()).unwrap();
    assert_eq!(
        placement_for(&reopened.preferences().display_placements, &setup),
        Some(setup[1])
    );
    // None removes the entry: the game opens on the main display again.
    let (saved, remembered) = reopened
        .save_display_options_on(DisplayOptions::default(), None, 1, &setup)
        .unwrap();
    assert!(!remembered);
    assert!(saved.display_placements.is_empty());
}

#[test]
fn an_empty_display_list_or_an_unlisted_monitor_leaves_the_placements_alone() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(dir.path()).unwrap();
    let setup = [display(0, 0, 1710, 1107)];
    state
        .save_display_options_on(DisplayOptions::default(), Some(rect(&setup[0])), 0, &setup)
        .unwrap();
    let before = state.preferences().display_placements.clone();
    for (revision, monitor, displays) in [
        (1, Some(at(0)), &[][..]),
        (2, None, &[][..]),
        (3, Some(at(4242)), &setup[..]),
    ] {
        let (saved, remembered) = state
            .save_display_options_on(DisplayOptions::default(), monitor, revision, displays)
            .unwrap();
        assert!(!remembered, "{revision}");
        assert_eq!(saved.display_placements, before, "{revision}");
    }
    // Revert proof: without the guard the empty list's key would be stored.
    assert!(before.iter().all(|p| p.setup != setup_key(&[])));
}

#[test]
fn a_preferences_file_from_before_the_placements_reads_an_empty_list() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("preferences.json"), OLD_FILE).unwrap();
    let state = DesktopState::open(dir.path()).unwrap();
    assert!(state.preferences().display_placements.is_empty());
}

#[test]
fn placements_are_stored_only_when_present_and_covered_by_the_digest() {
    let mut custom = Preferences::default();
    remember(
        &mut custom.display_placements,
        "0123456789abcdef",
        Some(at(0)),
    );
    let text = serde_json::to_string(&custom).unwrap();
    assert!(text.contains(
        r#""display_placements":[{"setup":"0123456789abcdef","x":0,"y":0,"width":1920,"height":1080}]"#
    ));
    assert_eq!(serde_json::from_str::<Preferences>(&text).unwrap(), custom);
    let default = serde_json::to_string(&Preferences::default().digest_form()).unwrap();
    let digested = serde_json::to_string(&custom.digest_form()).unwrap();
    assert!(!default.contains("display_placements"));
    assert!(digested.contains("display_placements"));
}
