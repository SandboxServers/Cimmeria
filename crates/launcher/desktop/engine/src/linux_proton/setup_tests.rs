#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::time::Instant;

use super::super::discovery::{ProtonInstall, ProtonKind};
use super::super::environment;
use super::super::prefix::{self, PhysxState, Prefix, Stamp};
use super::*;

const VERSION: &str = "1792432585 GE-Proton11-11";

/// A fake `proton` script in `dir` whose body runs after the shebang.
fn fake_proton(dir: &Path, body: &str) -> ProtonInstall {
    std::fs::create_dir_all(dir).unwrap();
    let script = dir.join("proton");
    std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    ProtonInstall {
        id: "ge:GE-Proton11-11-x86_64".into(),
        name: "GE-Proton11-11".into(),
        kind: ProtonKind::Ge,
        root: dir.to_path_buf(),
        version: VERSION.into(),
        runtime: None,
    }
}

/// Accepts only `run wineboot.exe` and creates `pfx/system.reg`, as wineboot does.
const WINEBOOT: &str = r#"if [ "$1" != run ] || [ "$2" != wineboot.exe ]; then exit 2; fi
mkdir -p "$STEAM_COMPAT_DATA_PATH/pfx" && : > "$STEAM_COMPAT_DATA_PATH/pfx/system.reg""#;

fn env_for(prefix: &Prefix) -> BTreeMap<OsString, OsString> {
    environment::base(
        |key| std::env::var_os(key),
        prefix,
        Path::new("/nonexistent"),
    )
}

fn stamp(version: &str, physx: PhysxState) -> Stamp {
    Stamp {
        schema_version: 1,
        proton_version: version.into(),
        physx,
    }
}

#[tokio::test]
async fn a_good_setup_writes_the_stamp_for_this_proton() {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    let install = fake_proton(dir.path(), WINEBOOT);
    ensure(&install, &prefix, &env_for(&prefix), None)
        .await
        .unwrap();
    assert!(prefix.pfx.join("system.reg").is_file());
    assert_eq!(
        prefix.stamp(),
        Some(stamp(VERSION, PhysxState::NotInstalled))
    );
}

#[tokio::test]
async fn a_failed_setup_is_a_prefix_error_and_writes_no_stamp() {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    let install = fake_proton(dir.path(), "exit 1");
    let result = ensure(&install, &prefix, &env_for(&prefix), None).await;
    assert!(matches!(result, Err(ProtonError::Setup("prefix"))));
    assert_eq!(prefix.stamp(), None);
}

#[tokio::test]
async fn exit_zero_without_system_reg_is_still_a_prefix_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    let install = fake_proton(dir.path(), "exit 0");
    let result = ensure(&install, &prefix, &env_for(&prefix), None).await;
    assert!(matches!(result, Err(ProtonError::Setup("prefix"))));
    assert_eq!(prefix.stamp(), None);
}

#[tokio::test]
async fn a_stamp_for_this_proton_skips_the_setup() {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    prefix
        .write_stamp(&stamp(VERSION, PhysxState::Installed))
        .unwrap();
    // The script would fail, so an Ok result proves it never ran.
    let install = fake_proton(dir.path(), "exit 1");
    ensure(&install, &prefix, &env_for(&prefix), None)
        .await
        .unwrap();
    assert!(!prefix.pfx.join("system.reg").exists());
}

#[tokio::test]
async fn a_stamp_for_another_proton_reruns_and_keeps_the_physx_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    prefix
        .write_stamp(&stamp("1 Proton 9.0", PhysxState::Installed))
        .unwrap();
    let install = fake_proton(dir.path(), WINEBOOT);
    ensure(&install, &prefix, &env_for(&prefix), None)
        .await
        .unwrap();
    assert_eq!(prefix.stamp(), Some(stamp(VERSION, PhysxState::Installed)));
}

#[tokio::test]
async fn setup_stderr_is_appended_to_the_log() {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    let log = root.path().join("setup.log");
    let install = fake_proton(dir.path(), "echo oops >&2; exit 1");
    let result = ensure(&install, &prefix, &env_for(&prefix), Some(&log)).await;
    assert!(matches!(result, Err(ProtonError::Setup("prefix"))));
    assert!(std::fs::read_to_string(&log).unwrap().contains("oops"));
}

#[tokio::test]
async fn a_setup_past_the_time_limit_is_killed_and_is_a_prefix_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    let install = fake_proton(dir.path(), "sleep 30");
    let started = Instant::now();
    let result = ensure_within(
        &install,
        &prefix,
        &env_for(&prefix),
        None,
        Duration::from_millis(200),
    )
    .await;
    assert!(matches!(result, Err(ProtonError::Setup("prefix"))));
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(prefix.stamp(), None);
}
