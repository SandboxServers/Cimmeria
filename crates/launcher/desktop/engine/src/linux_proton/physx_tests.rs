#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use super::super::discovery::{ProtonInstall, ProtonKind};
use super::super::environment;
use super::super::prefix::{self, PhysxState, Prefix, Stamp};
use super::*;

const VERSION: &str = "1792432585 GE-Proton11-11";

/// A probe report whose SDK initialized and released, as the worker prints it.
const PROBED_OK: &str = r#"{"schema_version":2,"architecture":"x86","activation_context":{"state":"loaded"},"modules":[{"component":"vc80_crt","result":{"state":"loaded"}},{"component":"vc80_cpp","result":{"state":"loaded"}},{"component":"d3dx9_40","result":{"state":"loaded"}},{"component":"xinput_1_3","result":{"state":"loaded"}},{"component":"physx_loader","result":{"state":"loaded"}}],"physx_sdk":{"state":"initialized_and_released"},"game_started":false}"#;

/// The same report, but the SDK could not be created.
const PROBED_NOT_INITIALIZED: &str = r#"{"schema_version":2,"architecture":"x86","activation_context":{"state":"loaded"},"modules":[{"component":"vc80_crt","result":{"state":"loaded"}},{"component":"vc80_cpp","result":{"state":"loaded"}},{"component":"d3dx9_40","result":{"state":"loaded"}},{"component":"xinput_1_3","result":{"state":"loaded"}},{"component":"physx_loader","result":{"state":"loaded"}}],"physx_sdk":{"state":"create_failed","sdk_error":null},"game_started":false}"#;

/// A fake `proton` whose `runinprefix` prints `result` (the `result` object of
/// a `PrepareResult`) for the request on stdin, echoing its operation and
/// generation as the host requires.
fn fake_worker(dir: &Path, result: &str) -> ProtonInstall {
    std::fs::create_dir_all(dir).unwrap();
    let script = dir.join("proton");
    let body = format!(
        r#"[ "$1" = runinprefix ] || exit 2
echo physx-stderr-marker >&2
req=$(cat)
op=$(printf '%s' "$req" | sed 's/.*"operation_id":"\([^"]*\)".*/\1/')
gen=$(printf '%s' "$req" | sed 's/.*"prefix_generation":"\([^"]*\)".*/\1/')
printf '%s' '{{"schema_version":1,"operation_id":"'"$op"'","prefix_generation":"'"$gen"'","result":{result}}}'"#
    );
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

/// The environment the worker gets: the prefix and a `PATH` for `sh`'s tools.
fn env_for(prefix: &Prefix) -> BTreeMap<OsString, OsString> {
    let mut env = environment::base(
        |key| std::env::var_os(key),
        prefix,
        Path::new("/nonexistent"),
    );
    env.insert("PATH".into(), std::env::var_os("PATH").unwrap());
    env
}

/// Runs `ensure_physx` once for a fresh prefix and a worker printing `result`.
/// The fixture folders live in the returned `TempDir`s, so they outlive the call.
async fn run_once(
    result: &str,
) -> (
    Result<(), ProtonError>,
    Prefix,
    tempfile::TempDir,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    let install = fake_worker(&dir.path().join("proton"), result);
    let game_binaries = root.path().join("game").join("Working").join("Binaries");
    std::fs::create_dir_all(&game_binaries).unwrap();
    let helper = dir.path().join("cimmeria-prerequisite-worker.exe");
    let outcome = ensure_physx(
        &install,
        &prefix,
        &env_for(&prefix),
        &helper,
        &game_binaries,
        Uuid::new_v4(),
        None,
    )
    .await;
    (outcome, prefix, dir, root)
}

/// A probed SDK that initialized and released stamps `Installed`, and a later
/// Play skips the worker. Reverting the skip (the `if` returning early in
/// `ensure_physx`) makes the second call run the failing worker and fail.
#[tokio::test]
async fn a_probed_and_released_sdk_is_installed_once() {
    let (first, prefix, _dir, _root) =
        run_once(&format!(r#"{{"state":"probed","report":{PROBED_OK}}}"#)).await;
    first.unwrap();
    assert_eq!(
        prefix.stamp(),
        Some(Stamp {
            schema_version: 1,
            proton_version: VERSION.into(),
            physx: PhysxState::Installed,
        })
    );
    // The worker would fail, so an Ok result proves it never ran.
    let dir = tempfile::tempdir().unwrap();
    let install = fake_worker(
        &dir.path().join("proton"),
        r#"{"state":"failed","reason":"io"}"#,
    );
    let game_binaries = dir.path().join("game").join("Working").join("Binaries");
    std::fs::create_dir_all(&game_binaries).unwrap();
    let again = ensure_physx(
        &install,
        &prefix,
        &env_for(&prefix),
        &dir.path().join("helper.exe"),
        &game_binaries,
        Uuid::new_v4(),
        None,
    )
    .await;
    again.unwrap();
}

/// An installer exit code stamps `Failed` and stops the Play with `physx`.
/// Reverting the `InstallerFailed` arm in `run_attempt` to `Ok` fails this test.
#[tokio::test]
async fn an_installer_failure_stamps_failed_and_stops_the_play() {
    let (outcome, prefix, _dir, _root) =
        run_once(r#"{"state":"installer_failed","installer_code":3010}"#).await;
    assert!(matches!(outcome, Err(ProtonError::Setup("physx"))));
    assert_eq!(
        prefix.stamp().map(|stamp| stamp.physx),
        Some(PhysxState::Failed)
    );
}

/// A failed probe, and a probe whose SDK did not initialize, are never taken as
/// installed. Reverting the `Failed` arm to `Ok` breaks the first case; reverting
/// the `physx_sdk == InitializedAndReleased` guard on the `Probed` arm breaks the
/// second.
#[tokio::test]
async fn a_failed_probe_or_an_uninitialized_sdk_is_never_installed() {
    let cases = [
        r#"{"state":"failed","reason":"probe"}"#.to_owned(),
        format!(r#"{{"state":"probed","report":{PROBED_NOT_INITIALIZED}}}"#),
    ];
    for result in cases {
        let (outcome, prefix, _dir, _root) = run_once(&result).await;
        assert!(
            matches!(outcome, Err(ProtonError::Setup("physx"))),
            "{result}"
        );
        assert_eq!(
            prefix.stamp().map(|stamp| stamp.physx),
            Some(PhysxState::Failed),
            "{result}"
        );
    }
}

/// A log file that cannot be opened never stops the install: stderr is discarded,
/// as the game launch does (PD-28). Reverting `stderr_for` to refuse fails this.
#[tokio::test]
async fn an_unwritable_log_does_not_stop_the_install() {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    let install = fake_worker(
        &dir.path().join("proton"),
        &format!(r#"{{"state":"probed","report":{PROBED_OK}}}"#),
    );
    let game_binaries = root.path().join("game").join("Working").join("Binaries");
    std::fs::create_dir_all(&game_binaries).unwrap();
    let log = dir.path().join("no-such-folder").join("physx.log");
    ensure_physx(
        &install,
        &prefix,
        &env_for(&prefix),
        &dir.path().join("cimmeria-prerequisite-worker.exe"),
        &game_binaries,
        Uuid::new_v4(),
        Some(&log),
    )
    .await
    .unwrap();
}

/// The worker's stderr is appended to the log file. Reverting `stderr_for` in
/// `prerequisites/supervisor.rs` to `Stdio::null()` leaves no file and fails this.
#[tokio::test]
async fn the_workers_stderr_is_appended_to_the_log() {
    let dir = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prefix = prefix::claim(root.path()).unwrap();
    let install = fake_worker(
        &dir.path().join("proton"),
        &format!(r#"{{"state":"probed","report":{PROBED_OK}}}"#),
    );
    let game_binaries = root.path().join("game").join("Working").join("Binaries");
    std::fs::create_dir_all(&game_binaries).unwrap();
    let log = dir.path().join("physx.log");
    ensure_physx(
        &install,
        &prefix,
        &env_for(&prefix),
        &dir.path().join("cimmeria-prerequisite-worker.exe"),
        &game_binaries,
        Uuid::new_v4(),
        Some(&log),
    )
    .await
    .unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(text.contains("physx-stderr-marker"), "{text}");
}
