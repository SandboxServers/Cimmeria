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
    assert!(
        !(std::env::var_os("SGW_PHYSX_CORE").is_some()
            && std::env::var_os("SGW_PHYSX_INSTALLER").is_some()),
        "choose core diagnostic or vendor installer, not both"
    );
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
    if let Some(installer) = std::env::var_os("SGW_PHYSX_INSTALLER") {
        if let Some(worker) = std::env::var_os("SGW_PREREQUISITE_WORKER") {
            run_prerequisite_worker(
                &runtime,
                &prefix,
                Path::new(&worker),
                Path::new(&installer),
                &game,
            )
            .await;
        } else {
            run_vendor_installer(&runtime, &prefix, Path::new(&installer)).await;
        }
        let report = run_probe(&runtime, &prefix, &helper, &game).await;
        assert_eq!(
            report.physx_sdk,
            cimmeria_runtime_probe::physx::SdkResult::InitializedAndReleased {}
        );
    }
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

/// Explicit headless experiment with the original vendor package; never used
/// by production admission until exit, SDK result and recovery are established.
async fn run_vendor_installer(runtime: &Path, prefix: &Path, source: &Path) {
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(
            File::open(source).unwrap(),
            crate::prerequisites::PHYSX_EXE_BYTES as u64 + 1,
        ),
        &mut bytes,
    )
    .unwrap();
    // Exercise the same authenticated extraction as the native coordinator will
    // use. Both modes authenticate the complete wrapper before any execution.
    // A same-size wrapper change must invalidate even an otherwise intact MSI.
    bytes[0] ^= 1;
    assert_eq!(
        crate::prerequisites::physx_msi(&bytes),
        Err(crate::prerequisites::PackageError::Identity)
    );
    bytes[0] ^= 1;
    let payload = crate::prerequisites::physx_msi(&bytes).unwrap();
    let mode = std::env::var("SGW_PHYSX_INSTALLER_MODE").unwrap_or_else(|_| "msi".into());
    assert!(matches!(mode.as_str(), "msi" | "exe"));
    let installer = prefix
        .parent()
        .unwrap()
        .join(format!("physx-7.11.13.{mode}"));
    if mode == "msi" {
        std::fs::write(&installer, payload).unwrap();
    } else {
        std::fs::write(&installer, bytes).unwrap();
    }
    let log = prefix.parent().unwrap().join("physx-msi.log");
    let mut env = environment(runtime, prefix).unwrap();
    env.insert("WINEDEBUG".into(), "-all,err+all".into());
    let mut command = tokio::process::Command::new(runtime.join("bin/wine"));
    if mode == "msi" {
        command
            .args(["msiexec", "/i"])
            .arg(paths::guest(&installer).unwrap())
            .args(["/qn", "/norestart", "REBOOT=ReallySuppress", "/l*v"])
            .arg(paths::guest(&log).unwrap());
    } else {
        command.arg(paths::guest(&installer).unwrap()).arg("/s");
    }
    let mut child = command
        .current_dir(prefix.parent().unwrap())
        .env_clear()
        .envs(&env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut errors = child.stderr.take().unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(180), async {
        let mut bytes = Vec::new();
        // Retain only a bounded diagnostic, but keep draining to avoid blocking
        // the installer or closing its pipe when the retained limit is reached.
        let mut chunk = [0u8; 4096];
        loop {
            let count = errors.read(&mut chunk).await?;
            if count == 0 {
                break;
            }
            let retain = count.min(16384usize.saturating_sub(bytes.len()));
            bytes.extend_from_slice(&chunk[..retain]);
        }
        let status = child.wait().await?;
        Ok::<_, std::io::Error>((status, bytes))
    })
    .await;
    let stopped = stop_prefix(runtime, &env).await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    stopped.unwrap();
    let (status, errors) = result.expect("bounded vendor installer").unwrap();
    eprintln!(
        "original PhysX vendor installer exit: {status}; diagnostic: {}",
        String::from_utf8_lossy(&errors)
    );
    if !status.success() {
        if let Ok(mut file) = File::open(&log) {
            let length = file.metadata().unwrap().len();
            std::io::Seek::seek(
                &mut file,
                std::io::SeekFrom::Start(length.saturating_sub(16384)),
            )
            .unwrap();
            let mut tail = Vec::new();
            std::io::Read::read_to_end(&mut std::io::Read::take(file, 16384), &mut tail).unwrap();
            eprintln!("MSI log tail: {}", String::from_utf8_lossy(&tail));
        }
    }
    assert!(status.success(), "vendor installer failed");
}

/// Exercise the native worker separately from the original msiexec experiment.
/// Parent admission/journaling are not provided by this disposable test harness.
async fn run_prerequisite_worker(
    runtime: &Path,
    prefix: &Path,
    worker: &Path,
    package: &Path,
    game: &Path,
) {
    use cimmeria_runtime_probe::prerequisite::{
        decode_result, PrepareRequest, ResultKind, MAX_RESULT,
    };
    let digest = std::env::var("SGW_PREREQUISITE_WORKER_SHA256").expect("worker artifact SHA256");
    let bytes = std::fs::read(worker).unwrap();
    assert_eq!(hex(&Sha256::digest(&bytes)), digest);
    let operation = Uuid::new_v4();
    let generation = Uuid::new_v4();
    let request = PrepareRequest {
        schema_version: 1,
        operation_id: operation,
        prefix_generation: generation,
        game_binaries: paths::guest(game).unwrap().into(),
        package: paths::guest(package).unwrap().into(),
        scratch: paths::guest(&prefix.parent().unwrap().join("worker-scratch"))
            .unwrap()
            .into(),
    };
    let env = environment(runtime, prefix).unwrap();
    let mut child = tokio::process::Command::new(runtime.join("bin/wine"))
        .arg(paths::guest(worker).unwrap())
        .current_dir(prefix.parent().unwrap())
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
    let attempt = tokio::time::timeout(std::time::Duration::from_secs(180), async {
        input
            .write_all(&serde_json::to_vec(&request).unwrap())
            .await?;
        drop(input);
        let mut bytes = Vec::new();
        output
            .take(MAX_RESULT as u64 + 1)
            .read_to_end(&mut bytes)
            .await?;
        let status = child.wait().await?;
        Ok::<_, std::io::Error>((status, bytes))
    })
    .await;
    let stopped = stop_prefix(runtime, &env).await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    stopped.unwrap();
    let (status, bytes) = attempt.expect("bounded prerequisite worker").unwrap();
    assert!(status.success(), "worker process: {status}");
    let result = decode_result(&bytes, operation, generation).unwrap();
    eprintln!("private prerequisite worker evidence: {result:?}");
    assert!(matches!(result.result, ResultKind::Probed { report }
        if report.physx_sdk == (cimmeria_runtime_probe::physx::SdkResult::InitializedAndReleased {})));
}
