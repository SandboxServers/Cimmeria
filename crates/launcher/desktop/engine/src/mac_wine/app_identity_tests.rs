use super::*;
use std::cell::Cell;

/// Inert files in the pinned runtime's layout. Nothing here is executed.
fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let runtime = root.join("runtime");
    for directory in [
        "bin",
        "lib/wine/x86_64-unix",
        "lib/wine/x86_64-windows",
        "lib/wine/i386-windows",
        "share/wine",
    ] {
        fs::create_dir_all(runtime.join(directory)).unwrap();
    }
    for (name, mode) in [("wine", 0o755), ("ntdll.so", 0o755), ("winemac.so", 0o644)] {
        let path = runtime.join("lib/wine/x86_64-unix").join(name);
        fs::write(&path, format!("inert {name}")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    }
    fs::write(runtime.join("bin/wine"), b"inert stock loader").unwrap();
    // The pinned runtime ships this one relative link in its loader directory.
    fs::create_dir(runtime.join("lib/external")).unwrap();
    fs::write(runtime.join("lib/external/libvulkan.1.dylib"), b"inert").unwrap();
    symlink(
        "../../external/libvulkan.1.dylib",
        runtime.join("lib/wine/x86_64-unix/libvulkan.1.dylib"),
    )
    .unwrap();
    let state = root.join("state");
    fs::create_dir(&state).unwrap();
    (temp, runtime, state)
}

fn outside_with_sentinel(state: &Path) -> (PathBuf, PathBuf) {
    let outside = state.parent().unwrap().join("outside");
    fs::create_dir(&outside).unwrap();
    let sentinel = outside.join("sentinel");
    fs::write(&sentinel, b"keep").unwrap();
    (outside, sentinel)
}

#[test]
fn staged_bundle_puts_the_real_loader_directory_inside_an_application_bundle() {
    let (_temp, runtime, state) = fixture();
    let bundle = stage(&state, &runtime, &GAME).unwrap();
    assert_eq!(bundle, state.join("wine-app-identity/Stargate Worlds.app"));
    let info = plist::Value::from_file(bundle.join("Contents/Info.plist"))
        .unwrap()
        .into_dictionary()
        .unwrap();
    let text = |key: &str| info.get(key).and_then(|value| value.as_string());
    assert_eq!(
        text("CFBundleIdentifier"),
        Some("app.cimmeria.stargate-worlds")
    );
    assert_eq!(text("CFBundleName"), Some("Stargate Worlds"));
    assert_eq!(text("CFBundleExecutable"), Some("wine"));
    assert_eq!(text("CFBundlePackageType"), Some("APPL"));
    // Without this every windowless Wine process becomes a second foreground app.
    assert_eq!(
        info.get("LSUIElement").and_then(|value| value.as_boolean()),
        Some(true)
    );
    let macos = bundle.join("Contents/MacOS");
    let unix = runtime.join("lib/wine/x86_64-unix");
    for (name, mode) in [("wine", 0o755), ("ntdll.so", 0o755), ("winemac.so", 0o644)] {
        let copy = macos.join(name);
        // Wine resolves its loader through realpath, so a link here would lose the bundle.
        let metadata = fs::symlink_metadata(&copy).unwrap();
        assert!(metadata.is_file(), "{name}");
        assert_eq!(metadata.permissions().mode() & 0o777, mode, "{name}");
        assert_eq!(fs::read(copy).unwrap(), fs::read(unix.join(name)).unwrap());
    }
    assert_eq!(
        fs::read_link(macos.join("x86_64-unix")).unwrap(),
        Path::new(".")
    );
    assert_eq!(
        fs::read_link(macos.join("libvulkan.1.dylib")).unwrap(),
        runtime.join("lib/external/libvulkan.1.dylib")
    );
    for name in ["x86_64-windows", "i386-windows"] {
        assert_eq!(
            fs::read_link(macos.join(name)).unwrap(),
            runtime.join("lib/wine").join(name)
        );
    }
    assert_eq!(
        fs::read_link(bundle.join("share")).unwrap(),
        runtime.join("share")
    );
    let entries = fs::read_dir(state.join("wine-app-identity"))
        .unwrap()
        .count();
    assert_eq!(entries, 1, "staging directory left behind");
}

#[test]
fn restaging_replaces_a_stale_bundle_without_following_its_links() {
    let (_temp, runtime, state) = fixture();
    let (outside, sentinel) = outside_with_sentinel(&state);
    let bundle = stage(&state, &runtime, &GAME).unwrap();
    let macos = bundle.join("Contents/MacOS");
    fs::write(macos.join("stale.so"), b"stale").unwrap();
    fs::write(macos.join("ntdll.so"), b"replaced").unwrap();
    symlink(&outside, macos.join("redirect")).unwrap();
    assert_eq!(stage(&state, &runtime, &GAME).unwrap(), bundle);
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
    assert!(fs::symlink_metadata(macos.join("stale.so")).is_err());
    assert!(fs::symlink_metadata(macos.join("redirect")).is_err());
    assert_eq!(fs::read(macos.join("ntdll.so")).unwrap(), b"inert ntdll.so");
}

#[test]
fn redirected_bundle_or_root_is_refused_and_its_target_left_alone() {
    let (_temp, runtime, state) = fixture();
    let (outside, sentinel) = outside_with_sentinel(&state);
    let root = state.join("wine-app-identity");
    fs::create_dir(&root).unwrap();
    let bundle = root.join("Stargate Worlds.app");
    symlink(&outside, &bundle).unwrap();
    assert!(stage(&state, &runtime, &GAME).is_err());
    assert_eq!(fs::read_link(&bundle).unwrap(), outside);
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");

    fs::remove_file(&bundle).unwrap();
    fs::remove_dir(&root).unwrap();
    symlink(&outside, &root).unwrap();
    assert!(stage(&state, &runtime, &GAME).is_err());
    assert_eq!(
        fs::read_dir(&outside).unwrap().count(),
        1,
        "wrote through the redirected root"
    );
}

#[test]
fn unexpected_runtime_layout_is_refused_without_publishing_a_bundle() {
    let bundle = Path::new("wine-app-identity/Stargate Worlds.app");

    let (_temp, runtime, state) = fixture();
    let unix = runtime.join("lib/wine/x86_64-unix");
    let (_, sentinel) = outside_with_sentinel(&state);
    symlink(&sentinel, unix.join("win32u.so")).unwrap();
    assert!(stage(&state, &runtime, &GAME).is_err());
    assert!(fs::symlink_metadata(state.join(bundle)).is_err());

    // A loader that is itself a link would run from the runtime, without the bundle.
    let (_temp, runtime, state) = fixture();
    let loader = runtime.join("lib/wine/x86_64-unix/wine");
    fs::rename(&loader, runtime.join("bin/real-loader")).unwrap();
    symlink(runtime.join("bin/real-loader"), &loader).unwrap();
    assert!(stage(&state, &runtime, &GAME).is_err());
    assert!(fs::symlink_metadata(state.join(bundle)).is_err());

    let (_temp, runtime, state) = fixture();
    let unix = runtime.join("lib/wine/x86_64-unix");
    let moved = runtime.join("lib/wine/moved");
    fs::rename(&unix, &moved).unwrap();
    symlink(&moved, &unix).unwrap();
    assert!(stage(&state, &runtime, &GAME).is_err());
    assert!(fs::symlink_metadata(state.join(bundle)).is_err());
}

#[test]
fn play_keeps_the_stock_loader_unless_identity_is_requested_and_staged() {
    let (_temp, runtime, state) = fixture();
    let stock = runtime.join("bin/wine");
    let registered = Cell::new(0);
    let register = |_: &Path| {
        registered.set(registered.get() + 1);
        Ok(())
    };
    let hosted = Cell::new(0);
    let host = |loader: &Path| {
        // The desktop host must never come from the bundle.
        assert_eq!(loader, stock);
        hosted.set(hosted.get() + 1);
        Ok("desktop host")
    };

    // Default Play: the stock loader, nothing written, nothing started.
    assert_eq!(
        select(false, &runtime, &state, register, host),
        (stock.clone(), None)
    );
    assert!(fs::symlink_metadata(state.join("wine-app-identity")).is_err());
    assert_eq!((registered.get(), hosted.get()), (0, 0));

    let staged = state.join("wine-app-identity/Stargate Worlds.app/Contents/MacOS/wine");
    assert_eq!(
        select(true, &runtime, &state, register, host),
        (staged.clone(), Some("desktop host"))
    );
    assert_eq!((registered.get(), hosted.get()), (1, 1));

    // A failed registration does not cost the staged identity.
    let failing = |_: &Path| Err(io::Error::other("fixture"));
    assert_eq!(select(true, &runtime, &state, failing, host).0, staged);

    // A runtime that cannot be staged still plays, with no desktop host started.
    fs::remove_file(runtime.join("lib/wine/x86_64-unix/ntdll.so")).unwrap();
    assert_eq!(
        select(true, &runtime, &state, register, host),
        (stock, None)
    );
    assert_eq!((registered.get(), hosted.get()), (1, 2));
}

#[test]
fn staged_loader_is_never_used_without_a_stock_desktop_host() {
    let (_temp, runtime, state) = fixture();
    // Otherwise the game starts the host from the bundle and two running
    // applications carry the identifier.
    let refused = |_: &Path| Err::<(), _>(io::Error::other("fixture"));
    assert_eq!(
        select(true, &runtime, &state, |_| Ok(()), refused),
        (runtime.join("bin/wine"), None)
    );
}

#[test]
fn only_an_explicit_one_opts_in() {
    assert!(requested(Some(OsStr::new("1"))));
    for value in ["", "0", "true", "yes", " 1"] {
        assert!(!requested(Some(OsStr::new(value))), "{value:?}");
    }
    assert!(!requested(None));
}
