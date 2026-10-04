//! Explicit headless validation only; not a production prerequisite coordinator.
use super::*;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
#[ignore = "requires native Windows x86 probe/hash and original client binaries; provisions private headless Wine"]
async fn original_client_module_probe_in_private_wine_prefix() {
    let helper =
        PathBuf::from(std::env::var_os("CIMMERIA_RUNTIME_PROBE").expect("set native x86 probe"));
    let digest = std::env::var("CIMMERIA_RUNTIME_PROBE_SHA256").expect("set artifact SHA256");
    assert_eq!(digest.len(), 64);
    let expected: Vec<u8> = digest
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect();
    verify_file(&helper, &expected.try_into().unwrap()).unwrap();
    let source =
        PathBuf::from(std::env::var_os("SGW_PROBE_BINARIES").expect("set original binaries"));
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let game = root_path.join("game");
    std::fs::create_dir(&game).unwrap();
    for (name, expected) in [
        (
            "SGW.exe",
            "b25adf3880256c6a6bab31594c0879c411ec0005ea4b7260aef991eaf8947e31",
        ),
        (
            "PhysXLoader.dll",
            "863e3ec87198bf1a5d5638a20695529dacc9460b0939f2579fe7a7faad2af924",
        ),
    ] {
        let bytes = std::fs::read(source.join(name)).unwrap();
        assert_eq!(hex(&Sha256::digest(&bytes)), expected);
        std::fs::write(game.join(name), bytes).unwrap();
    }
    let cache = root_path.join("runtimes");
    mac_runtime::prepare(
        cache.clone(),
        CancellationToken::new(),
        ProgressSink::latest().0,
    )
    .await
    .unwrap();
    let (runtime, _runtime_lock) = mac_runtime::verified_cached(&cache).unwrap();
    let prefix = root_path.join("bottle");
    std::fs::create_dir(&prefix).unwrap();
    std::fs::create_dir(prefix.join("drive_c")).unwrap();
    std::fs::create_dir(prefix.join("dosdevices")).unwrap();
    std::os::unix::fs::symlink("../drive_c", prefix.join("dosdevices/c:")).unwrap();
    std::os::unix::fs::symlink("/", prefix.join("dosdevices/z:")).unwrap();
    let report = run_probe(&runtime, &prefix, &helper, &game).await;
    assert_eq!(
        report.physx_sdk,
        cimmeria_runtime_probe::physx::SdkResult::CreateFailed { sdk_error: Some(1) }
    );
    if let Some(core) = std::env::var_os("SGW_PHYSX_CORE") {
        register_fixture_core(&runtime, &prefix, Path::new(&core)).await;
        let report = run_probe(&runtime, &prefix, &helper, &game).await;
        assert_eq!(
            report.physx_sdk,
            cimmeria_runtime_probe::physx::SdkResult::InitializedAndReleased {}
        );
    }
}

async fn run_probe(
    runtime: &Path,
    prefix: &Path,
    helper: &Path,
    game: &Path,
) -> cimmeria_runtime_probe::Report {
    let root_path = prefix.parent().unwrap();
    let env = environment(runtime, prefix).unwrap();
    let mut child = tokio::process::Command::new(runtime.join("bin/wine"))
        .arg(paths::guest(helper).unwrap())
        .current_dir(root_path)
        .env_clear()
        .envs(&env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let request = serde_json::to_vec(&serde_json::json!({
        "schema_version": 1, "game_binaries": paths::guest(game).unwrap()
    }))
    .unwrap();
    let attempt = tokio::time::timeout(std::time::Duration::from_secs(120), async {
        input.write_all(&request).await?;
        drop(input); // One-shot request ends at EOF.
        let mut bytes = Vec::new();
        output.take(8193).read_to_end(&mut bytes).await?;
        if bytes.len() > 8192 {
            return Err(std::io::Error::other("oversized probe report"));
        }
        let status = child.wait().await?;
        Ok::<_, std::io::Error>((status, bytes))
    })
    .await;
    // Run cleanup even after timeout/protocol/exit failure, before assertions.
    let stopped = stop_prefix(runtime, &env).await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    stopped.unwrap();
    let (status, bytes) = attempt.expect("bounded probe deadline").unwrap();
    assert!(status.success(), "probe exit: {status}");
    let report = cimmeria_runtime_probe::decode_report(&bytes).unwrap();
    assert_eq!(report.schema_version, 2);
    assert_eq!(report.architecture, "x86");
    assert!(!matches!(
        report.physx_sdk,
        cimmeria_runtime_probe::physx::SdkResult::NotChecked {}
    ));
    assert!(!report.game_started);
    let modules = &report.modules;
    assert_eq!(modules.len(), 5);
    assert_eq!(modules[4].component, "physx_loader");
    assert!(!String::from_utf8_lossy(&bytes).contains(&root_path.to_string_lossy().to_string()));
    eprintln!("private Wine module evidence: {report:?}");
    report
}

/// Diagnostic registration of the exact inertly extracted core. This deliberately
/// does not validate vendor installer behavior or replace production provisioning.
async fn register_fixture_core(runtime: &Path, prefix: &Path, source: &Path) {
    let bytes = std::fs::read(source).unwrap();
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "e54919c223e768e0fd12736119102069f7d3bdf1989f09f223119fd9ef0fe31e"
    );
    let root = prefix.parent().unwrap().join("physx-core");
    let version = root.join("v2.6.3");
    std::fs::create_dir_all(&version).unwrap();
    std::fs::write(version.join("PhysXCore.dll"), bytes).unwrap();
    let env = environment(runtime, prefix).unwrap();
    let command = tokio::process::Command::new(runtime.join("bin/wine"))
        .args([
            "reg",
            "add",
            r"HKLM\Software\Ageia Technologies",
            "/v",
            "PhysXCore Path",
            "/t",
            "REG_SZ",
            "/d",
        ])
        .arg(paths::guest(&root).unwrap())
        .args(["/f", "/reg:32"])
        .env_clear()
        .envs(&env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .status();
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), command).await;
    let stopped = stop_prefix(runtime, &env).await;
    stopped.unwrap();
    assert!(result.expect("bounded registry fixture").unwrap().success());
}
