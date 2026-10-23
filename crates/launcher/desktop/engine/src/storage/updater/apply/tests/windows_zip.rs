//! Windows zip swap (LX-13) against temporary install folders. The relaunch
//! is the injected `start`, so no launcher process is ever spawned.
use super::*;
use std::collections::BTreeMap;

const EXE: &str = "StargateWorlds-Launcher.exe";

pub(super) struct Fixture {
    pub(super) config: Config,
    pub(super) offer: Offer,
    pub(super) bytes: Vec<u8>,
    pub(super) target: InstalledTarget,
    pub(super) install: PathBuf,
}

fn signed(bytes: &[u8]) -> (Config, Offer) {
    let keys = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let signature = minisign::sign(
        Some(&keys.pk),
        &keys.sk,
        Cursor::new(bytes),
        Some("version:1.1.0"),
        None,
    )
    .unwrap();
    let config = Config::new(
        "https://updates.example.test/latest.json",
        &STANDARD.encode(keys.pk.to_box().unwrap().to_string()),
        "1.0.0",
        "windows-x86_64",
        vec!["updates.example.test".into()],
    )
    .unwrap();
    let offer = Offer {
        id: Uuid::new_v4(),
        version: "1.1.0".into(),
        notes: "Fixture zip".into(),
        url: "https://updates.example.test/StargateWorlds-Launcher-windows-x64.zip".into(),
        signature: STANDARD.encode(signature.to_string()),
    };
    (config, offer)
}

/// Release-shaped install (exe + windows/ + graphics/) plus a player file the
/// zip does not name, and a zip that replaces three files and adds one.
pub(super) fn windows_fixture(root: &Path) -> Fixture {
    let install = root.join("Launcher");
    for (name, bytes) in [
        (EXE, &b"old exe"[..]),
        ("windows/cimmeria-archive-worker.exe", b"old worker"),
        ("graphics/logo.png", b"old logo"),
        ("player-notes.txt", b"keep me"),
    ] {
        let path = install.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let bytes = zip_swap::zip_fixture(&[
        ("windows/", b""),
        ("windows/cimmeria-archive-worker.exe", b"new worker"),
        ("graphics/logo.png", b"new logo"),
        ("graphics/new-art/banner.png", b"added banner"),
        (EXE, b"new exe"),
    ]);
    with_zip(root, install, bytes)
}
fn with_zip(_root: &Path, install: PathBuf, bytes: Vec<u8>) -> Fixture {
    let (config, offer) = signed(&bytes);
    let target = InstalledTarget(Target::Windows {
        executable: install.join(EXE),
    });
    Fixture {
        config,
        offer,
        bytes,
        target,
        install,
    }
}
/// Every file under `dir`, `/`-separated, with its bytes.
pub(super) fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let name = path.strip_prefix(base).unwrap().to_string_lossy();
                out.insert(name.replace('\\', "/"), fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}
fn names(dir: &Path) -> Vec<String> {
    tree(dir).into_keys().collect()
}
/// The `.update-*` stage folders directly under `dir`, sorted.
pub(super) fn stages(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".update-"))
        .collect();
    names.sort();
    names
}
fn apply(
    state: &mut DesktopState,
    f: &Fixture,
    ready: &Snapshot,
    start: impl FnOnce(&Path, &[String]) -> Result<(), Error>,
) -> Result<Snapshot, Error> {
    state.apply_update_with(
        Some(&f.config),
        &f.target,
        ready.offer.as_ref().unwrap().id,
        ready.revision,
        0,
        start,
    )
}

#[test]
fn an_old_sidecar_install_upgrades_to_a_single_embedded_executable_zip() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let old = windows_fixture(&root);
    let f = with_zip(
        &root,
        old.install,
        zip_swap::zip_fixture(&[(EXE, b"new embedded launcher")]),
    );
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    let mut starts = 0;
    let status = apply(&mut state, &f, &ready, |path, args| {
        starts += 1;
        assert_eq!(fs::read(path).unwrap(), b"new embedded launcher");
        assert_eq!(args, [RESTART_ARG, &std::process::id().to_string()]);
        Ok(())
    })
    .unwrap();
    assert_eq!(starts, 1);
    assert_eq!(status.phase, Phase::RestartRequired);
    drop(state);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state.reconcile_launcher_update(&f.target, "1.1.0").unwrap();
    assert!(!f.install.join(format!("{EXE}.old")).exists());
    assert!(stages(&f.install).is_empty());
    // Migration must not guess which older sidecars or player files to delete.
    assert_eq!(
        fs::read(f.install.join("windows/cimmeria-archive-worker.exe")).unwrap(),
        b"old worker"
    );
    assert_eq!(
        fs::read(f.install.join("player-notes.txt")).unwrap(),
        b"keep me"
    );
}

#[test]
fn full_swap_installs_new_files_relaunches_once_and_next_start_cleans_up() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let f = windows_fixture(&root);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    let mut started = Vec::new();
    let status = apply(&mut state, &f, &ready, |path, args| {
        // The new exe is in place before the relaunch, under the running name.
        assert_eq!(fs::read(path).unwrap(), b"new exe");
        started.push((path.to_path_buf(), args.to_vec()));
        Ok(())
    })
    .unwrap();
    assert_eq!(status.phase, Phase::RestartRequired);
    assert_eq!(
        started,
        [(
            f.install.join(EXE),
            vec![RESTART_ARG.to_owned(), std::process::id().to_string()]
        )]
    );
    let expected_new = [
        (EXE, &b"new exe"[..]),
        ("graphics/logo.png", b"new logo"),
        ("graphics/new-art/banner.png", b"added banner"),
        ("player-notes.txt", b"keep me"),
        ("windows/cimmeria-archive-worker.exe", b"new worker"),
    ];
    let installed = tree(&f.install);
    for (name, bytes) in expected_new {
        assert_eq!(installed[name], bytes, "{name}");
    }
    // Until the new launcher acknowledges, the originals wait as `.old`.
    assert_eq!(installed[&format!("{EXE}.old")], b"old exe");
    assert_eq!(installed["graphics/logo.png.old"], b"old logo");
    assert_eq!(
        stages(&f.install).len(),
        1,
        "the stage waits for the next start"
    );
    drop(state);

    // An old launcher reopening cannot acknowledge and must not clean up.
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state.reconcile_launcher_update(&f.target, "1.0.0").unwrap();
    assert!(state.ensure_updater_idle().is_err());
    assert!(f.install.join(format!("{EXE}.old")).exists());
    drop(state);

    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state.reconcile_launcher_update(&f.target, "1.1.0").unwrap();
    assert_eq!(
        state
            .launcher_update_snapshot(Some(&f.config))
            .unwrap()
            .phase,
        Phase::Installed
    );
    assert!(state.ensure_updater_idle().is_ok());
    let expected: Vec<String> = expected_new.iter().map(|(n, _)| n.to_string()).collect();
    assert_eq!(names(&f.install), expected, "only the new files remain");
    assert!(stages(&f.install).is_empty());
}

#[test]
fn failure_on_the_third_rename_restores_every_original_byte_for_byte() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let f = windows_fixture(&root);
    let before = tree(&f.install);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    // Renames 1 and 2 move the worker aside and in; 3 would move the logo aside.
    zip_swap::FAIL_MOVE.with(|slot| slot.set(Some(3)));
    let error = apply(&mut state, &f, &ready, |_, _| {
        panic!("relaunch after a failed swap")
    })
    .unwrap_err();
    assert_eq!(error, Error::Replace);
    assert_eq!(
        tree(&f.install),
        before,
        "installed files restored byte for byte"
    );
    assert!(!f.install.join("graphics/new-art").exists());
    assert!(stages(&f.install).is_empty());
    let snapshot = state.launcher_update_snapshot(Some(&f.config)).unwrap();
    assert_eq!(snapshot.phase, Phase::Failed);
    assert_eq!(snapshot.failure, Some(Error::Replace));
    assert!(state.ensure_updater_idle().is_ok());
}

#[test]
fn failed_relaunch_after_a_complete_swap_restores_the_originals() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let f = windows_fixture(&root);
    let before = tree(&f.install);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    assert_eq!(
        apply(&mut state, &f, &ready, |_, _| Err(Error::Spawn)).unwrap_err(),
        Error::Spawn
    );
    assert_eq!(tree(&f.install), before);
    assert!(state.ensure_updater_idle().is_ok());
}

#[test]
fn a_start_with_leftover_old_files_cleans_only_the_planned_ones() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let f = windows_fixture(&root);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    apply(&mut state, &f, &ready, |_, _| Ok(())).unwrap();
    let attempt = state.update_record().unwrap().apply.unwrap();
    let plan = attempt.swap.unwrap();
    drop(state);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state.reconcile_launcher_update(&f.target, "1.1.0").unwrap();
    drop(state);
    // The acknowledging start could not delete them (an antivirus scan, say).
    let stage = f.install.join(attempt.staged_bundle.unwrap());
    fs::create_dir(&stage).unwrap();
    fs::write(
        stage.join(".cimmeria-owner"),
        zip_swap::plan_owner(&plan).to_string(),
    )
    .unwrap();
    fs::write(f.install.join(format!("{EXE}.old")), b"old exe").unwrap();
    fs::write(
        f.install.join("windows/cimmeria-archive-worker.exe.old"),
        b"old",
    )
    .unwrap();
    // Not a file the update replaced: never the updater's to delete.
    fs::write(f.install.join("player-notes.txt.old"), b"player backup").unwrap();

    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state.reconcile_launcher_update(&f.target, "1.1.0").unwrap();
    assert!(!f.install.join(format!("{EXE}.old")).exists());
    assert!(!f
        .install
        .join("windows/cimmeria-archive-worker.exe.old")
        .exists());
    assert!(!stage.exists());
    assert_eq!(
        fs::read(f.install.join("player-notes.txt.old")).unwrap(),
        b"player backup"
    );
    assert_eq!(
        state
            .launcher_update_snapshot(Some(&f.config))
            .unwrap()
            .phase,
        Phase::Installed
    );
}

#[test]
fn unsafe_zip_entries_are_refused_before_any_write() {
    let mut cases: Vec<(&str, Vec<u8>)> = [
        "../evil.txt",
        "windows/../../evil.txt",
        "/evil.txt",
        "C:/evil.txt",
        "windows\\..\\..\\evil.txt",
        "graphics/NUL.png",
        "graphics/trailing.png.",
        ".update-1.1.0/evil.txt",
        "windows/worker.exe.old",
        // 8.3 short names alias a long name already on disk.
        "graphics/LOGO~1.PNG",
        "WINDOW~2/worker.exe",
        "windows/CONIN$",
        "graphics/conout$.txt",
        "windows/COM\u{b9}.dll",
        "graphics/lpt\u{b3}.png",
    ]
    .into_iter()
    .map(|name| {
        (
            name,
            zip_swap::zip_fixture(&[
                (EXE, b"new exe"),
                ("graphics/logo.png", b"x"),
                (name, b"evil"),
            ]),
        )
    })
    .collect();
    let mut link = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    link.start_file(EXE, options).unwrap();
    std::io::Write::write_all(&mut link, b"new exe").unwrap();
    link.add_symlink("graphics/escape", "../../outside", options)
        .unwrap();
    cases.push(("symlink", link.finish().unwrap().into_inner()));
    cases.push((
        "two root executables",
        zip_swap::zip_fixture(&[(EXE, b"new exe"), ("other.exe", b"second")]),
    ));
    for (name, bytes) in cases {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let f = windows_fixture(&root);
        let f = with_zip(&root, f.install, bytes);
        let before = tree(&f.install);
        let mut state = DesktopState::open(&root.join("state")).unwrap();
        let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
        assert_eq!(
            apply(&mut state, &f, &ready, |_, _| panic!("relaunch for {name}")).unwrap_err(),
            Error::Package,
            "{name}"
        );
        assert_eq!(tree(&f.install), before, "{name}: install untouched");
        assert!(stages(&f.install).is_empty(), "{name}: no stage");
        assert!(!root.join("evil.txt").exists() && !root.join("outside").exists());
        assert!(state.ensure_updater_idle().is_ok(), "{name}");
    }
}

#[test]
fn names_that_only_resemble_reserved_ones_are_installed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let f = windows_fixture(&root);
    let near = [
        "graphics/COM10.png",
        "graphics/a~b1.png",
        "graphics/console.png",
    ];
    let mut entries: Vec<(&str, &[u8])> = vec![(EXE, b"new exe")];
    entries.extend(near.iter().map(|name| (*name, &b"art"[..])));
    let f = with_zip(&root, f.install, zip_swap::zip_fixture(&entries));
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    apply(&mut state, &f, &ready, |_, _| Ok(())).unwrap();
    let installed = tree(&f.install);
    for name in near {
        assert_eq!(installed[name], b"art", "{name}");
    }
}

#[test]
fn a_directory_where_a_file_goes_is_refused_with_nothing_moved() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let f = windows_fixture(&root);
    fs::create_dir_all(f.install.join("graphics/new-art/banner.png")).unwrap();
    let before = tree(&f.install);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    assert_eq!(
        apply(&mut state, &f, &ready, |_, _| panic!("relaunch")).unwrap_err(),
        Error::Target
    );
    assert_eq!(tree(&f.install), before);
    assert!(stages(&f.install).is_empty());
    assert!(state.ensure_updater_idle().is_ok());
}

#[test]
fn installer_packages_are_refused_on_windows() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut f = windows_fixture(&root);
    f.offer.url = "https://updates.example.test/launcher-setup.exe".into();
    let before = tree(&f.install);
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    assert_eq!(
        apply(&mut state, &f, &ready, |_, _| panic!("installer handoff")).unwrap_err(),
        Error::Package
    );
    assert_eq!(tree(&f.install), before);
}
