use super::*;
use std::collections::BTreeMap;

fn launcher_env() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("DISPLAY", ":0"),
        ("HOME", "/home/deck"),
        ("PROTON_LOG", "1"),
        ("SteamAppId", "1"),
        ("LD_PRELOAD", "/tmp/evil.so"),
        ("WINEPREFIX", "/somewhere"),
        ("APPDIR", "/tmp/app"),
        ("APPIMAGE", "/tmp/app.AppImage"),
        ("OWD", "/tmp"),
    ])
}

fn get<'a>(
    env: &'a BTreeMap<&'static str, &'static str>,
) -> impl Fn(&str) -> Option<OsString> + 'a {
    move |key| env.get(key).map(|value| OsString::from(*value))
}

fn prefix_in(root: &Path) -> Prefix {
    super::super::prefix::claim(root).unwrap()
}

#[test]
fn base_copies_display_but_not_the_proton_or_steam_debug_variables() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_in(root.path());
    let env = launcher_env();
    let base = base(get(&env), &prefix, Path::new("/home/deck/.steam/root"));
    assert_eq!(
        base.get(&OsString::from("DISPLAY")),
        Some(&OsString::from(":0"))
    );
    assert_eq!(
        base.get(&OsString::from("HOME")),
        Some(&OsString::from("/home/deck"))
    );
    for absent in [
        "PROTON_LOG",
        "SteamAppId",
        "LD_PRELOAD",
        "WINEPREFIX",
        "APPDIR",
        "APPIMAGE",
        "OWD",
    ] {
        assert!(
            !base.contains_key(&OsString::from(absent)),
            "{absent} leaked"
        );
    }
}

#[test]
fn base_copies_every_passthrough_variable() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_in(root.path());
    let mut env: BTreeMap<&'static str, &'static str> =
        PASSTHROUGH.iter().map(|&key| (key, "v")).collect();
    env.insert("APPIMAGE", "leak");
    env.insert("OWD", "leak");
    env.insert("STEAM_COMPAT_DATA_PATH", "leak");
    let base = base(get(&env), &prefix, Path::new("/steam"));
    for &key in PASSTHROUGH {
        assert_eq!(
            base.get(&OsString::from(key)),
            Some(&OsString::from("v")),
            "{key} was not copied"
        );
    }
    for absent in ["APPIMAGE", "OWD"] {
        assert!(
            !base.contains_key(&OsString::from(absent)),
            "{absent} leaked"
        );
    }
    assert_eq!(
        base.get(&OsString::from("STEAM_COMPAT_DATA_PATH")),
        Some(&prefix.compat.clone().into_os_string())
    );
}

#[test]
fn base_sets_the_steam_paths_and_wine_overrides() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_in(root.path());
    let env = BTreeMap::new();
    let base = base(get(&env), &prefix, Path::new("/home/deck/.steam/root"));
    assert_eq!(
        base.get(&OsString::from("STEAM_COMPAT_DATA_PATH")),
        Some(&prefix.compat.clone().into_os_string())
    );
    assert_eq!(
        base.get(&OsString::from("STEAM_COMPAT_CLIENT_INSTALL_PATH")),
        Some(&OsString::from("/home/deck/.steam/root"))
    );
    assert_eq!(
        base.get(&OsString::from("WINEDLLOVERRIDES")),
        Some(&OsString::from("winemenubuilder.exe=d;mscoree=d;mshtml=d"))
    );
}

#[test]
fn game_sets_the_frame_cap_and_log_variables_only_with_a_log_folder() {
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix_in(root.path());
    let env = BTreeMap::new();
    let plain = game(base(get(&env), &prefix, Path::new("/steam")), 60, None);
    assert_eq!(
        plain.get(&OsString::from("DXVK_FRAME_RATE")),
        Some(&OsString::from("60"))
    );
    assert!(!plain.contains_key(&OsString::from("DXVK_LOG_LEVEL")));
    assert!(!plain.contains_key(&OsString::from("DXVK_LOG_PATH")));

    let logged = game(
        base(get(&env), &prefix, Path::new("/steam")),
        144,
        Some("Z:\\logs"),
    );
    assert_eq!(
        logged.get(&OsString::from("DXVK_LOG_LEVEL")),
        Some(&OsString::from("info"))
    );
    assert_eq!(
        logged.get(&OsString::from("DXVK_LOG_PATH")),
        Some(&OsString::from("Z:\\logs"))
    );
}
