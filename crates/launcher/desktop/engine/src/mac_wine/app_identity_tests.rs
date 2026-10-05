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
    let bundle = stage(&state, &runtime).unwrap();
    assert_eq!(bundle, state.join("wine-app-identity/Stargate Worlds.app"));
    let info = plist::Value::from_file(bundle.join("Contents/Info.plist"))
        .unwrap()
        .into_dictionary()
        .unwrap();
    let text = |key: &str| info.get(key).and_then(|value| value.as_string());
    assert_eq!(text("CFBundleIdentifier"), Some(BUNDLE_IDENTIFIER));
    assert_eq!(text("CFBundleName"), Some(DISPLAY_NAME));
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
    let bundle = stage(&state, &runtime).unwrap();
    let macos = bundle.join("Contents/MacOS");
    fs::write(macos.join("stale.so"), b"stale").unwrap();
    fs::write(macos.join("ntdll.so"), b"replaced").unwrap();
    symlink(&outside, macos.join("redirect")).unwrap();
    assert_eq!(stage(&state, &runtime).unwrap(), bundle);
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
    assert!(stage(&state, &runtime).is_err());
    assert_eq!(fs::read_link(&bundle).unwrap(), outside);
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep");

    fs::remove_file(&bundle).unwrap();
    fs::remove_dir(&root).unwrap();
    symlink(&outside, &root).unwrap();
    assert!(stage(&state, &runtime).is_err());
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
    assert!(stage(&state, &runtime).is_err());
    assert!(fs::symlink_metadata(state.join(bundle)).is_err());

    // A loader that is itself a link would run from the runtime, without the bundle.
    let (_temp, runtime, state) = fixture();
    let loader = runtime.join("lib/wine/x86_64-unix/wine");
    fs::rename(&loader, runtime.join("bin/real-loader")).unwrap();
    symlink(runtime.join("bin/real-loader"), &loader).unwrap();
    assert!(stage(&state, &runtime).is_err());
    assert!(fs::symlink_metadata(state.join(bundle)).is_err());

    let (_temp, runtime, state) = fixture();
    let unix = runtime.join("lib/wine/x86_64-unix");
    let moved = runtime.join("lib/wine/moved");
    fs::rename(&unix, &moved).unwrap();
    symlink(&moved, &unix).unwrap();
    assert!(stage(&state, &runtime).is_err());
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

    // Default Play: the stock loader, and nothing written.
    assert_eq!(select(false, &runtime, &state, register), stock);
    assert!(fs::symlink_metadata(state.join("wine-app-identity")).is_err());
    assert_eq!(registered.get(), 0);

    let staged = state.join("wine-app-identity/Stargate Worlds.app/Contents/MacOS/wine");
    assert_eq!(select(true, &runtime, &state, register), staged);
    assert_eq!(registered.get(), 1);

    // A failed registration does not cost the staged identity.
    let failing = |_: &Path| Err(io::Error::other("fixture"));
    assert_eq!(select(true, &runtime, &state, failing), staged);

    // A runtime that cannot be staged still plays.
    fs::remove_file(runtime.join("lib/wine/x86_64-unix/ntdll.so")).unwrap();
    assert_eq!(select(true, &runtime, &state, register), stock);
    assert_eq!(registered.get(), 1);
}

#[test]
fn only_an_explicit_one_opts_in() {
    assert!(requested(Some(OsStr::new("1"))));
    for value in ["", "0", "true", "yes", " 1"] {
        assert!(!requested(Some(OsStr::new(value))), "{value:?}");
    }
    assert!(!requested(None));
}

/// Launch Services application type and Wine command line of each running process
/// whose main bundle is `bundle`.
fn running(bundle: &Path) -> Vec<(String, String)> {
    let run = |program: &str, arguments: &[&str]| {
        let output = std::process::Command::new(program)
            .args(arguments)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    let identifier = format!("bundleID=\"{BUNDLE_IDENTIFIER}\"");
    let path = format!("bundle path=\"{}\"", bundle.display());
    let (mut ours, mut here, mut processes) = (false, false, Vec::new());
    for line in run("/usr/bin/lsappinfo", &["list"]).lines() {
        if !line.starts_with(' ') {
            (ours, here) = (false, false);
        }
        let line = line.trim();
        ours |= line == identifier;
        here |= line == path;
        if ours && here && line.starts_with("pid = ") {
            let mut fields = line.split_whitespace().skip(2);
            let pid = fields.next().unwrap_or_default();
            let kind = line
                .split("type=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next());
            // Wine retitles each process to its Windows command line.
            let command = run("/bin/ps", &["-o", "args=", "-p", pid]);
            processes.push((kind.unwrap_or("unknown").to_owned(), command));
        }
    }
    processes
}

/// Real pinned Wine, isolated: a clone of the runtime, a throwaway prefix and Wine's
/// own Notepad. Never the game, the saved installation or its prefix. Stops only the
/// wineserver of that throwaway prefix.
#[test]
#[ignore = "runs real Wine from CIMMERIA_WINE_RUNTIME_CLONE and shows a Notepad window"]
fn real_wine_window_owner_carries_the_bundle_identity() {
    let runtime = std::env::var_os("CIMMERIA_WINE_RUNTIME_CLONE")
        .map(PathBuf::from)
        .expect("CIMMERIA_WINE_RUNTIME_CLONE: a copy of the managed runtime directory")
        .canonicalize()
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let bundle = stage(&root, &runtime).unwrap();
    let prefix = root.join("home/bottle");
    fs::create_dir_all(&prefix).unwrap();
    let mut environment = environment(&runtime, &prefix).unwrap();
    // The interactive policy: Wine's Mac driver stays enabled, as in Play.
    environment.insert(
        "WINEDLLOVERRIDES".into(),
        "winemenubuilder.exe,mscoree,mshtml=d".into(),
    );
    let wine = |program: PathBuf, argument: &str| {
        std::process::Command::new(program)
            .arg(argument)
            .env_clear()
            .envs(&environment)
            .current_dir(&root)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    // A 32-bit parent starting a 32-bit child, the shape of the launch worker and SGW.exe.
    let mut guest = std::process::Command::new(bundle.join("Contents/MacOS/wine"))
        .args([r"C:\windows\syswow64\cmd.exe", "/c", "start", "/wait"])
        .arg(r"C:\windows\syswow64\notepad.exe")
        .env_clear()
        .envs(&environment)
        .current_dir(&root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let foreground = |processes: &[(String, String)]| {
        processes
            .iter()
            .filter(|(kind, _)| kind == "Foreground")
            .map(|(_, command)| command.clone())
            .collect::<Vec<_>>()
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    let processes = loop {
        let processes = running(&bundle);
        // Wine's Mac driver promotes a process only while it shows a window; creating
        // the prefix shows a progress window first.
        let windows = foreground(&processes);
        let settled = windows.len() == 1 && windows[0].contains("notepad.exe");
        if settled || std::time::Instant::now() > deadline {
            break processes;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    };
    wine(runtime.join("bin/wineserver"), "-k").wait().unwrap();
    guest.wait().unwrap();
    // Launch Services records a bundle once a process from it checks in.
    let _ = std::process::Command::new(LSREGISTER)
        .arg("-u")
        .arg(&bundle)
        .status();
    let windows = foreground(&processes);
    assert_eq!(windows.len(), 1, "{processes:?}");
    assert!(
        windows[0].contains(r"syswow64\notepad.exe"),
        "{processes:?}"
    );
    // The desktop host shares the bundle but must stay out of the foreground.
    assert!(
        processes
            .iter()
            .any(|(kind, command)| kind == "UIElement" && command.contains("explorer.exe")),
        "{processes:?}"
    );
}
