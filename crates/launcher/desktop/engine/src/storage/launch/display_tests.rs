//! The display step: pure transforms, the helper arguments and `apply` on a
//! temp profile (C-PD7, D-PD16).
use super::*;
use crate::test_capture::{capture, Captured};
use tracing::Level;

const SAVED: &str = concat!(
    "<options valueInt=\"3\" version=\"1\" optionName=\"resolution\" groupName=\"video\"></options>\r\n",
    "<options valueBool=\"false\" version=\"1\" optionName=\"windowedMode\" groupName=\"video\"></options>\r\n",
    "<options valueBool=\"false\" version=\"1\" optionName=\"windowedMode\" groupName=\"audio\"></options>\r\n",
    "<options valueFloat=\"0.5\" version=\"1\" optionName=\"gamma\" groupName=\"video\"></options>\r\n",
);

const INI: &str = concat!(
    "[IniVersion]\r\n",
    "0=1234567890.000000\r\n",
    "[WinDrv.WindowsClient]\r\n",
    "startupfullscreen=True\r\n",
    "StartupResolutionX=1024\t\r\n",
    "StartupResolutionY=600\r\n",
    "[Engine.Other]\r\n",
    "StartupFullscreen=True\r\n",
);

fn windowed(width: u32, height: u32) -> DisplayOptions {
    DisplayOptions {
        mode: DisplayMode::Windowed,
        width,
        height,
        ..DisplayOptions::default()
    }
}

#[test]
fn saved_options_flips_only_the_windowed_mode_value_and_keeps_crlf() {
    let out = saved_options(SAVED, true).unwrap();
    let before: Vec<&str> = SAVED.split_inclusive('\n').collect();
    let after: Vec<&str> = out.split_inclusive('\n').collect();
    assert_eq!(after.len(), 4);
    assert_eq!(after[0], before[0]);
    assert_eq!(
        after[1],
        "<options valueBool=\"true\" version=\"1\" optionName=\"windowedMode\" groupName=\"video\"></options>\r\n"
    );
    assert_eq!(after[2], before[2], "another group's windowedMode is kept");
    assert_eq!(after[3], before[3]);
    assert_eq!(saved_options(&out, false).unwrap(), SAVED);
}

#[test]
fn saved_options_without_the_line_or_with_two_is_none() {
    let without = SAVED.replace(
        r#"optionName="windowedMode" groupName="video""#,
        r#"optionName="vsync" groupName="video""#,
    );
    assert_eq!(saved_options(&without, true), None);
    let twice = format!("{SAVED}{}", SAVED.split_inclusive('\n').nth(1).unwrap());
    assert_eq!(saved_options(&twice, true), None);
}

#[test]
fn engine_ini_changes_only_the_windows_client_keys() {
    let out = engine_ini(INI, false, Some((1600, 900))).unwrap();
    assert_eq!(
        out,
        concat!(
            "[IniVersion]\r\n",
            "0=1234567890.000000\r\n",
            "[WinDrv.WindowsClient]\r\n",
            "startupfullscreen=False\r\n",
            "StartupResolutionX=1600\r\n",
            "StartupResolutionY=900\r\n",
            "[Engine.Other]\r\n",
            "StartupFullscreen=True\r\n",
        )
    );
    // The same size drops the chain file's trailing tab.
    let same = engine_ini(INI, false, Some((1024, 600))).unwrap();
    assert!(same.contains("\r\nStartupResolutionX=1024\r\n"), "{same:?}");
    // Fullscreen leaves the size alone.
    let full = engine_ini(INI, true, None).unwrap();
    assert!(full.contains("\r\nstartupfullscreen=True\r\nStartupResolutionX=1024\t\r\n"));
}

#[test]
fn engine_ini_keeps_lf_and_refuses_a_missing_section_or_key() {
    let lf = INI.replace("\r\n", "\n");
    let out = engine_ini(&lf, false, Some((800, 600))).unwrap();
    assert!(!out.contains('\r'));
    assert!(out.contains("\nStartupResolutionX=800\n"));
    assert_eq!(engine_ini("[IniVersion]\n0=1\n", false, None), None);
    let no_y = INI.replace("StartupResolutionY=600\r\n", "");
    assert_eq!(engine_ini(&no_y, false, Some((800, 600))), None);
    assert!(
        engine_ini(&no_y, true, None).is_some(),
        "fullscreen needs no size"
    );
}

#[test]
fn arguments_follow_the_effective_mode() {
    assert_eq!(
        arguments(windowed(1600, 900)),
        ["-windowed", "ResX=1600", "ResY=900"]
    );
    let fullscreen = DisplayOptions {
        mode: DisplayMode::Fullscreen,
        ..windowed(1600, 900)
    };
    #[cfg(not(target_os = "macos"))]
    assert!(arguments(fullscreen).is_empty());
    #[cfg(target_os = "macos")]
    assert_eq!(
        arguments(fullscreen),
        ["-windowed", "ResX=1600", "ResY=900"]
    );
}

fn profile(ini: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(SAVED_OPTIONS), SAVED).unwrap();
    if let Some(ini) = ini {
        std::fs::create_dir(dir.path().join("Config")).unwrap();
        std::fs::write(dir.path().join(ENGINE_INI), ini).unwrap();
    }
    dir
}

fn applied(profile: Option<&Path>, display: DisplayOptions) -> Captured {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let ((), events) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        apply(Uuid::new_v4(), profile, display)
    });
    let mut matching = events
        .into_iter()
        .filter(|e| e.field("event") == Some("display_settings"));
    let event = matching.next().expect("one display_settings event");
    assert!(matching.next().is_none());
    assert_eq!(event.target, "launcher.launch");
    assert!(event.field("operation").is_some());
    event
}

#[test]
fn apply_writes_both_files_then_reports_unchanged_without_touching_them() {
    let dir = profile(Some(INI));
    let first = applied(Some(dir.path()), windowed(1600, 900));
    assert_eq!(first.level, Level::INFO);
    assert_eq!(first.field("mode"), Some("windowed"));
    assert_eq!(first.field("saved_options"), Some("written"));
    assert_eq!(first.field("engine_ini"), Some("written"));
    assert_eq!(first.field("failure_code"), None);
    let ini = std::fs::read_to_string(dir.path().join(ENGINE_INI)).unwrap();
    assert!(ini.contains("startupfullscreen=False\r\nStartupResolutionX=1600\r\n"));
    let saved = std::fs::read_to_string(dir.path().join(SAVED_OPTIONS)).unwrap();
    assert_eq!(saved, saved_options(SAVED, true).unwrap());

    let stamp = |name: &str| std::fs::metadata(dir.path().join(name)).unwrap();
    let before = [stamp(SAVED_OPTIONS), stamp(ENGINE_INI)];
    let second = applied(Some(dir.path()), windowed(1600, 900));
    assert_eq!(second.level, Level::INFO);
    assert_eq!(second.field("saved_options"), Some("unchanged"));
    assert_eq!(second.field("engine_ini"), Some("unchanged"));
    let after = [stamp(SAVED_OPTIONS), stamp(ENGINE_INI)];
    for (b, a) in before.iter().zip(&after) {
        assert_eq!(b.modified().unwrap(), a.modified().unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(b.ino(), a.ino(), "not replaced");
        }
    }
}

#[test]
fn a_missing_ini_is_reported_and_never_created() {
    let dir = profile(None);
    let event = applied(Some(dir.path()), windowed(1600, 900));
    assert_eq!(event.level, Level::WARN);
    assert_eq!(event.field("saved_options"), Some("written"));
    assert_eq!(event.field("engine_ini"), Some("missing"));
    assert_eq!(
        event.field("failure_code"),
        Some("display_settings_not_applied")
    );
    assert!(!dir.path().join("Config").exists());
    let names: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(names.len(), 1, "no temp file is left behind");
}

#[test]
fn no_profile_reports_both_missing_and_a_utf16_file_is_unreadable() {
    let event = applied(None, windowed(1600, 900));
    assert_eq!(event.level, Level::WARN);
    assert_eq!(event.field("saved_options"), Some("missing"));
    assert_eq!(event.field("engine_ini"), Some("missing"));

    let dir = profile(Some(INI));
    let utf16: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain(SAVED.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    std::fs::write(dir.path().join(SAVED_OPTIONS), &utf16).unwrap();
    let event = applied(Some(dir.path()), windowed(1600, 900));
    assert_eq!(event.field("saved_options"), Some("unreadable"));
    assert_eq!(event.field("engine_ini"), Some("written"));
    assert_eq!(
        std::fs::read(dir.path().join(SAVED_OPTIONS)).unwrap(),
        utf16
    );
}

#[test]
fn with_no_saved_preference_play_writes_windowed_1024x768() {
    let (_root, state, _plan) = super::super::tests::fixture();
    let display = options(&Arc::new(Mutex::new(state)));
    assert_eq!(display, windowed(1024, 768));
    let dir = profile(Some(
        &INI.replace("StartupResolutionX=1024\t", "StartupResolutionX=800"),
    ));
    let event = applied(Some(dir.path()), display);
    assert_eq!(event.field("mode"), Some("windowed"));
    let ini = std::fs::read_to_string(dir.path().join(ENGINE_INI)).unwrap();
    assert!(ini.contains(concat!(
        "startupfullscreen=False\r\n",
        "StartupResolutionX=1024\r\n",
        "StartupResolutionY=768\r\n"
    )));
}

#[cfg(target_os = "macos")]
#[test]
fn wine_profile_finds_the_one_user_and_ignores_public() {
    let prefix = tempfile::tempdir().unwrap();
    let users = prefix.path().join("drive_c/users");
    let tail = "Documents/My Games/Firesky/SGWGame";
    std::fs::create_dir_all(users.join("Public").join(tail)).unwrap();
    std::fs::create_dir_all(users.join("player").join(tail)).unwrap();
    let expected = users.join("player").join(tail).canonicalize().unwrap();
    assert_eq!(wine_profile(prefix.path()), Some(expected));

    // A Documents link out of the prefix is refused.
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(outside.path().join("My Games/Firesky/SGWGame")).unwrap();
    std::fs::remove_dir_all(users.join("player/Documents")).unwrap();
    std::os::unix::fs::symlink(outside.path(), users.join("player/Documents")).unwrap();
    assert_eq!(wine_profile(prefix.path()), None);

    // Two user folders: no guess.
    std::fs::create_dir_all(users.join("second").join(tail)).unwrap();
    assert_eq!(wine_profile(prefix.path()), None);
}
