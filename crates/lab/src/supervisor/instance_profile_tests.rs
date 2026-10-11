use super::*;

#[test]
fn only_an_explicit_yes_shares_the_folder() {
    for yes in ["1", "true", "yes", " 1 ", "TRUE", "Yes"] {
        assert!(shared_user_dir_from(Some(yes)), "{yes:?}");
    }
    for no in [None, Some(""), Some("0"), Some("false"), Some("no")] {
        assert!(!shared_user_dir_from(no), "{no:?}");
    }
}

#[test]
fn profile_lives_under_the_root() {
    let p = profile_dir(Path::new("C:/cimmeria-lab/instances"), "p3");
    assert!(p.ends_with("cimmeria-lab/instances/p3/profile"));
}

#[test]
fn profile_root_prefers_the_override_then_local_app_data() {
    let local = Path::new(r"C:\Users\tester\AppData\Local");
    let default = local.join("cimmeria-lab").join("instances");
    assert_eq!(
        profile_root_from(Some(r" D:\lab\profiles "), Some(local)),
        Some(PathBuf::from(r"D:\lab\profiles"))
    );
    for raw in [None, Some(""), Some("   ")] {
        assert_eq!(
            profile_root_from(raw, Some(local)),
            Some(default.clone()),
            "{raw:?}"
        );
    }
    assert_eq!(profile_root_from(None, None), None);
    assert_eq!(profile_root_from(Some(""), None), None);
}

#[test]
fn inside_ignores_case_and_needs_a_component_boundary() {
    let install = Path::new(r"c:\games\sgw");
    assert!(inside(Path::new(r"C:\Games\SGW\binaries\x"), install));
    assert!(inside(Path::new(r"C:\Games\SGW"), install));
    assert!(inside(Path::new("C:/Games/SGW/Binaries"), install));
    assert!(!inside(Path::new(r"C:\Games\SGW2"), install));
    assert!(!inside(Path::new(r"C:\Games"), install));
    assert!(!inside(Path::new(r"D:\Games\SGW\x"), install));
}

#[test]
fn inside_normalises_dots_verbatim_prefixes_and_non_ascii_case() {
    let install = Path::new(r"C:\Games\SGW");
    // `..` pops: this path is the install's own sibling folder walked back in.
    assert!(inside(Path::new(r"C:\Games\Other\..\SGW\x"), install));
    assert!(!inside(Path::new(r"C:\Games\SGW\..\Other"), install));
    // `.` is dropped.
    assert!(inside(Path::new(r"C:\Games\.\SGW\x"), install));
    // A verbatim prefix matches its plain drive form.
    assert!(inside(Path::new(r"\\?\C:\Games\SGW\x"), install));
    assert!(inside(Path::new(r"\\?\c:\games\sgw"), install));
    assert!(!inside(Path::new(r"\\?\C:\Games\SGW2"), install));
    // Lowercasing is not ASCII-only.
    assert!(inside(
        Path::new(r"C:\Jeux\ÉTÉ\x"),
        Path::new(r"c:\jeux\été")
    ));
}

#[test]
fn inside_treats_an_empty_dir_as_containing_everything() {
    assert!(inside(Path::new(r"C:\Games\SGW\x"), Path::new("")));
    assert!(!inside(Path::new(""), Path::new(r"C:\Games\SGW")));
}

#[test]
fn userprofile_relative_documents_are_redirectable() {
    let home = Path::new("C:/Users/tester");
    for raw in [
        r"%USERPROFILE%\Documents",
        r"%userprofile%\Documents",
        "%USERPROFILE%/Documents",
        r" %USERPROFILE%\Documents\ ",
    ] {
        assert_eq!(
            redirectable_documents(raw, home).unwrap(),
            home.join("Documents"),
            "{raw:?}"
        );
    }
}

#[test]
fn absolute_documents_are_not_redirectable() {
    let home = Path::new("C:/Users/tester");
    for raw in [
        r"C:\Users\tester\OneDrive\Documents",
        r"D:\Docs",
        r"%OneDrive%\Documents",
        "",
    ] {
        let err = redirectable_documents(raw, home).unwrap_err();
        assert!(err.contains("#1312"), "{raw:?}: {err}");
    }
}

fn write(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

#[test]
fn seed_copies_config_content_cache_and_top_files_only() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("real/SGWGame");
    write(&src.join("Cache.en-US/TextStrings.pak"), "pak");
    write(&src.join("Config/SGWEngine.ini"), "ini");
    write(&src.join("Content/LocalShaderCache-PC-D3D-SM3.upk"), "upk");
    write(&src.join("SavedSystemOptions.xml"), "xml");
    write(
        &src.join("lab/Labone/ActionButtons - Saved Vars.lua"),
        "account",
    );
    write(&src.join("Logs/Launch.log"), "log");
    write(&src.join("CrashDumps/x.dmp"), "dump");
    let profile = tmp.path().join("profile");

    let out = seed(&src, &profile).unwrap();
    assert_eq!(
        out,
        Seeded::Copied {
            files: 4,
            bytes: 12
        }
    );
    let dst = sgwgame_dir(&profile.join("Documents"));
    for kept in [
        "Cache.en-US/TextStrings.pak",
        "Config/SGWEngine.ini",
        "Content/LocalShaderCache-PC-D3D-SM3.upk",
        "SavedSystemOptions.xml",
    ] {
        assert!(dst.join(kept).is_file(), "{kept} must be seeded");
    }
    for left in ["lab", "Logs", "CrashDumps"] {
        assert!(!dst.join(left).exists(), "{left} must not be seeded");
    }
    assert!(!dst.with_file_name("SGWGame.seeding").exists());
}

#[test]
fn seed_runs_once_so_the_instance_keeps_its_own_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("real/SGWGame");
    write(&src.join("Cache.en-US/CookedWorldInfo.pak"), "v1");
    let profile = tmp.path().join("profile");
    seed(&src, &profile).unwrap();
    let mine = sgwgame_dir(&profile.join("Documents")).join("Cache.en-US/CookedWorldInfo.pak");
    std::fs::write(&mine, "updated by this instance's login").unwrap();
    write(&src.join("Cache.en-US/CookedWorldInfo.pak"), "v2");

    assert_eq!(seed(&src, &profile).unwrap(), Seeded::AlreadyThere);
    assert_eq!(
        std::fs::read_to_string(&mine).unwrap(),
        "updated by this instance's login"
    );
}

#[test]
fn seed_without_a_real_folder_starts_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let profile = tmp.path().join("profile");
    assert_eq!(
        seed(&tmp.path().join("missing"), &profile).unwrap(),
        Seeded::NoSource
    );
    assert!(sgwgame_dir(&profile.join("Documents")).is_dir());
}

#[test]
fn an_interrupted_seed_is_redone() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("real/SGWGame");
    write(&src.join("Config/SGWEngine.ini"), "ini");
    let profile = tmp.path().join("profile");
    let half = sgwgame_dir(&profile.join("Documents")).with_file_name("SGWGame.seeding");
    write(&half.join("partial.tmp"), "x");

    assert_eq!(
        seed(&src, &profile).unwrap(),
        Seeded::Copied { files: 1, bytes: 3 }
    );
    assert!(!half.exists());
    assert!(
        !sgwgame_dir(&profile.join("Documents"))
            .join("partial.tmp")
            .exists(),
        "a half-finished seed's files must not reach the instance"
    );
}

#[test]
fn prepare_points_the_default_instance_at_its_own_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("real/SGWGame");
    write(&src.join("Config/SGWEngine.ini"), "ini");
    let install = tmp.path().join("install");
    let root = tmp.path().join("profiles");
    let env = prepare_from(
        false,
        || Ok(src.clone()),
        Some(root.clone()),
        &install,
        "default",
        None,
    )
    .unwrap();
    let profile = profile_dir(&root, "default");
    assert_eq!(
        env,
        (
            USER_PROFILE_ENV.to_string(),
            profile.to_string_lossy().into_owned()
        )
    );
    assert!(sgwgame_dir(&profile.join("Documents"))
        .join("Config/SGWEngine.ini")
        .is_file());
}

#[test]
fn prepare_falls_back_to_the_shared_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("install");
    let src = tmp.path().join("real/SGWGame");
    let root = tmp.path().join("profiles");
    write(&src.join("Config/SGWEngine.ini"), "ini");

    // Opted out: the real folder is not even looked up.
    assert_eq!(
        prepare_from(
            true,
            || panic!("not asked"),
            Some(root.clone()),
            &install,
            "p2",
            None
        ),
        None
    );
    // Documents not redirectable (OneDrive, a registry error).
    assert_eq!(
        prepare_from(
            false,
            || Err("absolute".into()),
            Some(root.clone()),
            &install,
            "p2",
            None
        ),
        None
    );
    // The seed fails: the profile's Documents is a file.
    write(&profile_dir(&root, "p3").join("Documents"), "not a dir");
    assert_eq!(
        prepare_from(
            false,
            || Ok(src.clone()),
            Some(root.clone()),
            &install,
            "p3",
            None
        ),
        None
    );
}

#[test]
fn prepare_refuses_an_unsafe_root() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("install");
    let src = tmp.path().join("real/SGWGame");
    write(&src.join("Config/SGWEngine.ini"), "ini");

    let inside_root = install.join("Binaries/sessions/instances");
    assert_eq!(
        prepare_from(
            false,
            || Ok(src.clone()),
            Some(inside_root),
            &install,
            "p2",
            None
        ),
        None
    );
    // No root at all (no override, no LOCALAPPDATA): also shared.
    assert_eq!(
        prepare_from(false, || Ok(src.clone()), None, &install, "p2", None),
        None
    );
    // A relative root would resolve against the game's cwd (Binaries).
    let relative = PathBuf::from(r"profiles\instances");
    assert_eq!(
        prepare_from(
            false,
            || Ok(src.clone()),
            Some(relative),
            &install,
            "p2",
            None
        ),
        None
    );
    // An empty LOCALAPPDATA gives the same relative default root.
    let from_empty_local = profile_root_from(None, Some(Path::new(""))).unwrap();
    assert!(!from_empty_local.is_absolute());
    assert_eq!(
        prepare_from(
            false,
            || Ok(src.clone()),
            Some(from_empty_local),
            &install,
            "p2",
            None
        ),
        None
    );
}

const STOCK_INI: &str = "[TextureStreaming]\r\nHysteresisLimit=10\r\n";
const OVERRIDDEN_INI: &str = "[TextureStreaming]\r\nHysteresisLimit=99\r\n";

/// A real `SGWGame` with a stock ini, an overrides file setting
/// `HysteresisLimit = 99`, and the instance's profile root.
fn overrides_fixture(tmp: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let src = tmp.join("real/SGWGame");
    write(&src.join("Config/SGWEngine.ini"), STOCK_INI);
    let overrides = tmp.join("ini-overrides.toml");
    write(
        &overrides,
        "[[override]]\nfile = \"SGWEngine.ini\"\nsection = \"TextureStreaming\"\n\
         key = \"HysteresisLimit\"\nvalue = 99\n",
    );
    (src, overrides, tmp.join("profiles"))
}

fn profile_ini(root: &Path, label: &str) -> PathBuf {
    sgwgame_dir(&profile_dir(root, label).join("Documents")).join("Config/SGWEngine.ini")
}

fn launch(src: &Path, root: &Path, install: &Path, overrides: &Path) -> Option<(String, String)> {
    prepare_from(
        false,
        || Ok(src.to_path_buf()),
        Some(root.to_path_buf()),
        install,
        "p2",
        Some(overrides.to_path_buf()),
    )
}

#[test]
fn prepare_applies_the_overrides_to_a_new_seed() {
    let tmp = tempfile::tempdir().unwrap();
    let (src, overrides, root) = overrides_fixture(tmp.path());
    let install = tmp.path().join("install");

    assert!(launch(&src, &root, &install, &overrides).is_some());
    assert_eq!(
        std::fs::read_to_string(profile_ini(&root, "p2")).unwrap(),
        OVERRIDDEN_INI
    );
    // Only the instance's copy changes, never the real ini.
    assert_eq!(
        std::fs::read_to_string(src.join("Config/SGWEngine.ini")).unwrap(),
        STOCK_INI
    );
}

#[test]
fn prepare_applies_the_overrides_on_a_later_launch_and_after_a_reseed() {
    let tmp = tempfile::tempdir().unwrap();
    let (src, overrides, root) = overrides_fixture(tmp.path());
    let install = tmp.path().join("install");
    let ini = profile_ini(&root, "p2");
    launch(&src, &root, &install, &overrides).unwrap();

    // A later launch (`AlreadyThere`) after the client regenerated its ini.
    std::fs::write(&ini, STOCK_INI).unwrap();
    launch(&src, &root, &install, &overrides).unwrap();
    assert_eq!(std::fs::read_to_string(&ini).unwrap(), OVERRIDDEN_INI);

    // A reseed: the profile's SGWGame is deleted and copied again.
    std::fs::remove_dir_all(sgwgame_dir(&profile_dir(&root, "p2").join("Documents"))).unwrap();
    launch(&src, &root, &install, &overrides).unwrap();
    assert_eq!(std::fs::read_to_string(&ini).unwrap(), OVERRIDDEN_INI);
}

#[test]
fn a_bad_overrides_file_applies_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let (src, overrides, root) = overrides_fixture(tmp.path());
    let install = tmp.path().join("install");
    // The second entry is refused, so the valid first one is not applied either.
    write(
        &overrides,
        "[[override]]\nfile = \"SGWEngine.ini\"\nsection = \"TextureStreaming\"\n\
         key = \"HysteresisLimit\"\nvalue = 99\n\
         [[override]]\nfile = \"SGWEngine.ini\"\nsection = \"IniVersion\"\nkey = \"0\"\nvalue = 1\n",
    );
    assert!(launch(&src, &root, &install, &overrides).is_some());
    assert_eq!(
        std::fs::read_to_string(profile_ini(&root, "p2")).unwrap(),
        STOCK_INI
    );
}

#[test]
fn a_new_empty_profile_gets_the_overrides_from_its_second_launch() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, overrides, root) = overrides_fixture(tmp.path());
    let missing = tmp.path().join("no-real-folder");
    let install = tmp.path().join("install");
    let ini = profile_ini(&root, "p2");

    assert!(launch(&missing, &root, &install, &overrides).is_some());
    assert!(!ini.exists());
    // The client writes its ini on that first run.
    write(&ini, STOCK_INI);
    launch(&missing, &root, &install, &overrides).unwrap();
    assert_eq!(std::fs::read_to_string(&ini).unwrap(), OVERRIDDEN_INI);
}

#[test]
fn a_shared_folder_fallback_never_applies_the_overrides() {
    let tmp = tempfile::tempdir().unwrap();
    let (src, overrides, root) = overrides_fixture(tmp.path());
    let install = tmp.path().join("install");
    // The seed fails: the profile's Documents is a file.
    write(&profile_dir(&root, "p2").join("Documents"), "not a dir");
    assert_eq!(launch(&src, &root, &install, &overrides), None);
    assert_eq!(
        std::fs::read_to_string(src.join("Config/SGWEngine.ini")).unwrap(),
        STOCK_INI
    );
}
