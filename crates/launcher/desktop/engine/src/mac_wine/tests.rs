use super::*;
use crate::{
    catalog::{verify_release, VerifiedRelease},
    AdmissionRequest, OperationState,
};
use ed25519_dalek::{Signer, SigningKey};
fn digest(path: &Path) -> [u8; 32] {
    Sha256::digest(std::fs::read(path).unwrap()).into()
}
fn fixture() -> (
    tempfile::TempDir,
    Arc<Mutex<DesktopState>>,
    Uuid,
    VerifiedRelease,
) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("game")), false, 0)
        .unwrap();
    let body=serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"fixture","size":1,"sha256":"00".repeat(32)},"patches":[]})).unwrap();
    let sig = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    let hex: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let release = verify_release(&body, hex.as_bytes()).unwrap();
    let id = Uuid::new_v4();
    state
        .admit_install_backend(AdmissionRequest {
            id,
            operation_revision: 0,
            preferences_revision: 1,
            release: &release,
            login_servers: vec![],
            backend: ExtractionBackend::Wine {
                runtime_sha256: [1; 32],
                helper_sha256: [2; 32],
            },
        })
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    (root, Arc::new(Mutex::new(state)), id, release)
}
#[test]
fn guest_paths_preserve_unicode_and_spaces_without_shell_quoting() {
    assert_eq!(
        paths::guest(Path::new("/Volumes/Game Disk/星門/seed.rar")).unwrap(),
        "Z:\\Volumes\\Game Disk\\星門\\seed.rar"
    );
    for bad in [
        "relative",
        "/x/../y",
        "/bad\\name",
        "/bad:name",
        "/bad?name",
        "/ends. ",
        "/trailing.",
        "/CON",
        "/aux.txt",
        "/COM1.zip",
        "/LPT²",
        "/NUL .txt",
    ] {
        assert!(paths::guest(Path::new(bad)).is_err(), "{bad}");
    }
}
#[test]
fn prefix_is_exclusive_and_never_silently_reused() {
    let (root, state, id, _) = fixture();
    let intent = state.lock().unwrap().install_intent().unwrap().unwrap();
    let cache = root.path().canonicalize().unwrap().join("prefixes");
    let (prefix, guard) = claim_prefix(&cache, &intent).unwrap();
    assert_eq!(
        std::fs::read_link(prefix.join("dosdevices/z:")).unwrap(),
        Path::new("/")
    );
    assert!(prefix.join("drive_c").is_dir());
    assert_eq!(
        std::fs::read_link(prefix.join("dosdevices/c:")).unwrap(),
        Path::new("../drive_c")
    );
    let saved: InstallIntent = serde_json::from_slice(
        &std::fs::read(cache.join(id.to_string()).join("owner.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved, intent);
    assert!(claim_prefix(&cache, &intent).is_err());
    drop(guard);
    assert!(claim_prefix(&cache, &intent).is_err());
}
#[test]
fn helper_identity_rejects_corruption_and_symlinks() {
    let root = tempfile::tempdir().unwrap();
    let helper = root.path().join("helper.exe");
    std::fs::write(&helper, b"helper").unwrap();
    let hash = digest(&helper);
    verify_file(&helper, &hash).unwrap();
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(&helper, &alias).unwrap();
    assert!(verify_file(&alias, &hash).is_err());
    std::fs::write(&helper, b"bad").unwrap();
    assert!(verify_file(&helper, &hash).is_err());
}
#[test]
fn extraction_environment_is_headless_and_does_not_inherit_host_values() {
    let (root, state, _id, _) = fixture();
    let intent = state.lock().unwrap().install_intent().unwrap().unwrap();
    let (prefix, owner) = claim_prefix(
        &root.path().canonicalize().unwrap().join("prefixes"),
        &intent,
    )
    .unwrap();
    let adapter = WineSeedExtractor {
        state,
        intent,
        runtime: PathBuf::from("/runtime"),
        helper: PathBuf::from("/bundle/helper.exe"),
        prefix: prefix.clone(),
        _owner: owner,
        used: std::sync::atomic::AtomicBool::new(false),
        limits: Deadlines::default(),
    };
    let command = adapter.command().unwrap();
    assert_eq!(command.executable, Path::new("/runtime/bin/wine"));
    assert_eq!(
        command.arguments,
        vec![OsString::from("Z:\\bundle\\helper.exe")]
    );
    assert_eq!(
        command.environment.get(&OsString::from("WINEPREFIX")),
        Some(&prefix.into_os_string())
    );
    assert!(command.environment[&OsString::from("WINEDLLOVERRIDES")]
        .to_str()
        .unwrap()
        .contains("winemac.drv"));
    assert!(!command.environment.contains_key(&OsString::from("DISPLAY")));
}
#[tokio::test]
async fn invalid_runtime_identity_fails_before_provisioning_or_prefix_creation() {
    let (root, state, id, _) = fixture();
    assert!(matches!(
        WineSeedExtractor::prepare(
            state,
            id,
            PathBuf::from("/missing"),
            CancellationToken::new(),
            ProgressSink::latest().0
        )
        .await,
        Err(WineError::Invalid)
    ));
    assert!(!root.path().join("state/runtimes").exists());
    assert!(!root.path().join("state/wine-prefixes").exists());
}

#[tokio::test]
async fn duplicate_extraction_cannot_stop_the_active_prefix() {
    use std::os::unix::fs::PermissionsExt;
    let (root, state, id, _) = fixture();
    let intent = state.lock().unwrap().install_intent().unwrap().unwrap();
    let (prefix, owner) = claim_prefix(
        &root.path().canonicalize().unwrap().join("prefixes"),
        &intent,
    )
    .unwrap();
    let runtime = root.path().join("runtime");
    std::fs::create_dir_all(runtime.join("bin")).unwrap();
    let event = serde_json::to_string(&crate::archive_worker::WorkerEvent {
        schema_version: 1,
        operation_id: Some(id),
        event: crate::archive_worker::EventKind::Finished { error: None },
    })
    .unwrap();
    let script = format!(
        r#"#!/bin/sh
read request
: > "$WINEPREFIX/started"
while [ ! -f "$WINEPREFIX/release" ]; do /bin/sleep 0.01; done
printf '%s\n' '{}'
"#,
        event
    );
    for (name, content) in [
        ("wine", script.as_str()),
        (
            "wineserver",
            "#!/bin/sh\necho stop >> \"$WINEPREFIX/stops\"\n",
        ),
    ] {
        let path = runtime.join("bin").join(name);
        std::fs::write(&path, content).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let cache = intent.destination.join(format!(".cimmeria-cache-{id}"));
    let stage = intent.destination.join(format!(".cimmeria-stage-{id}"));
    std::fs::create_dir_all(&cache).unwrap();
    let archive = cache.join("seed.zip");
    let adapter = WineSeedExtractor {
        state,
        intent,
        runtime,
        helper: PathBuf::from("/fixture/helper.exe"),
        prefix: prefix.clone(),
        _owner: owner,
        used: std::sync::atomic::AtomicBool::new(false),
        limits: Deadlines::default(),
    };
    let hash = "00".repeat(32);
    let request = || SeedExtraction {
        archive: &archive,
        destination: &stage,
        sha256: &hash,
        cancel: CancellationToken::new(),
        progress: ProgressSink::latest().0,
    };
    let first = adapter.extract(request());
    tokio::pin!(first);
    tokio::select! {
        result = &mut first => panic!("first extraction exited before release: {result:?}"),
        _ = async {
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while !prefix.join("started").exists() {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }).await.unwrap();
        } => ()
    }
    assert!(matches!(
        adapter.extract(request()).await,
        Err(InstallError::SeedExtractionUncertain)
    ));
    assert!(
        !prefix.join("stops").exists(),
        "duplicate must not stop the active prefix"
    );
    std::fs::write(prefix.join("release"), b"").unwrap();
    first.await.unwrap();
    assert_eq!(
        std::fs::read_to_string(prefix.join("stops"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}
