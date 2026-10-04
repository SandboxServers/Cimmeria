use super::*;
use std::ffi::OsStr;

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Graphics) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let runtime = root.join("runtime");
    std::fs::create_dir_all(runtime.join("lib/vulkan/icd.d")).unwrap();
    std::fs::write(
        runtime.join("lib/vulkan/icd.d/MoltenVK_icd.json"),
        br#"{"ICD":{"library_path":"../../external/libMoltenVK.dylib"}}"#,
    )
    .unwrap();
    let overlay = root.join("d3d9.dll");
    std::fs::write(&overlay, b"inert overlay").unwrap();
    let graphics = Graphics {
        d3d9: Artifact::open(
            overlay,
            &Sha256::digest(b"inert overlay")
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        )
        .unwrap(),
        rosetta_x87: None,
    };
    (temp, runtime, root.join("game-prefix/bottle"), graphics)
}

#[test]
fn game_environment_selects_bundled_vulkan_without_changing_headless_policy() {
    let (_temp, runtime, prefix, graphics) = fixture();
    let environment = game_environment(&runtime, &prefix, &graphics).unwrap();
    assert_eq!(
        environment.get(OsStr::new("VK_DRIVER_FILES")),
        Some(
            &runtime
                .join("lib/vulkan/icd.d/MoltenVK_icd.json")
                .into_os_string()
        )
    );
    assert_eq!(
        environment.get(OsStr::new("WINEPREFIX")),
        Some(&prefix.into_os_string())
    );
    assert_eq!(
        environment.get(OsStr::new("WINEDLLOVERRIDES")).unwrap(),
        "d3d9=n;winemenubuilder.exe,mscoree,mshtml=d"
    );
    assert_eq!(
        environment.get(OsStr::new("CX_FWD_COMPAT_GL_CTX")).unwrap(),
        "1"
    );
    assert_eq!(
        environment.get(OsStr::new("DXVK_FRAME_RATE")).unwrap(),
        "30"
    );
    assert!(!environment.contains_key(OsStr::new("ROSETTA_X87_PATH")));
    let headless = mac_wine::environment(&runtime, Path::new("/fixture/bottle")).unwrap();
    assert!(!headless.contains_key(OsStr::new("VK_DRIVER_FILES")));
    assert!(!headless.contains_key(OsStr::new("DXVK_FRAME_RATE")));
    assert!(headless
        .get(OsStr::new("WINEDLLOVERRIDES"))
        .unwrap()
        .to_str()
        .unwrap()
        .contains("winemac.drv"));
}

#[test]
fn missing_or_nonordinary_descriptor_refuses_game_environment_before_spawn() {
    let (_temp, runtime, prefix, graphics) = fixture();
    let descriptor = runtime.join("lib/vulkan/icd.d/MoltenVK_icd.json");
    std::fs::remove_file(&descriptor).unwrap();
    assert!(matches!(
        game_environment(&runtime, &prefix, &graphics),
        Err(IntentError::Storage(StorageError::UnsafeFile))
    ));
    std::fs::create_dir(&descriptor).unwrap();
    assert!(game_environment(&runtime, &prefix, &graphics).is_err());
    std::fs::remove_dir(&descriptor).unwrap();
    let outside = runtime.parent().unwrap().join("outside.json");
    std::fs::write(&outside, b"{}").unwrap();
    std::os::unix::fs::symlink(&outside, &descriptor).unwrap();
    assert!(game_environment(&runtime, &prefix, &graphics).is_err());
}

#[test]
fn redirected_descriptor_ancestor_refuses_game_environment_before_spawn() {
    let (_temp, runtime, prefix, graphics) = fixture();
    let directory = runtime.join("lib/vulkan/icd.d");
    let outside = runtime.parent().unwrap().join("outside");
    std::fs::rename(&directory, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &directory).unwrap();
    assert!(game_environment(&runtime, &prefix, &graphics).is_err());
}
