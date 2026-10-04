//! Isolated legacy-launcher inputs for adoption journeys, shared by engine and
//! host tests. Inert signed ZIP content under a private temporary root: never a
//! real game, account, prefix or installation.
use crate::{
    adoption, catalog::VerifiedRelease, client_setup, install_progress::ProgressSink, migration,
    unpack, DesktopState,
};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Deliberately not alphabetical: the reviewed order must survive adoption.
pub const SERVERS: [(&str, &str); 2] = [
    ("Second", "https://example.invalid/second"),
    ("First", "https://example.invalid/first"),
];
pub const LEGACY_INSTALL_ID: &str = "72a8a13b-2a4e-4ea0-b5ba-5ba3cf0a619d";
pub const AUTH_URL: &str = "https://example.invalid/auth";
/// Signed minimum of every fixture release; `older_launcher` is blocked by it.
pub const MINIMUM: &str = "launcher-20261004-fixture";

pub struct Legacy {
    pub root: tempfile::TempDir,
    pub source: migration::LegacySource,
    /// Exact bytes of the legacy configuration the user wrote.
    pub config: Vec<u8>,
    seed: PathBuf,
    release: VerifiedRelease,
}

impl Legacy {
    /// An old launcher folder and a separate old game folder holding the signed
    /// seed's content, with the given reviewed patch and game-telemetry settings.
    pub fn new(client_patches: bool, telemetry_opted_in: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let launcher = root.path().join("legacy");
        let game = root.path().join("old-game");
        std::fs::create_dir(&launcher).unwrap();
        std::fs::create_dir(&game).unwrap();
        let bytes = crate::install_worker::fixtures::archive(true);
        let seed = root.path().join("seed.zip");
        std::fs::write(&seed, &bytes).unwrap();
        unpack::unpack(
            &seed,
            &game,
            &unpack::UnpackSink {
                progress: ProgressSink::latest().0,
                label: "fixture".into(),
                cancel: CancellationToken::new(),
            },
        )
        .unwrap();
        let servers: Vec<_> = SERVERS
            .iter()
            .map(|(name, url)| client_setup::LoginServer {
                name: (*name).into(),
                url: (*url).into(),
            })
            .collect();
        client_setup::prepare(&game, &servers).unwrap();
        let config = serde_json::to_vec_pretty(&serde_json::json!({
            "install_path": "C:\\Old Game",
            "manifest_url": crate::catalog::URL,
            "login_servers": SERVERS.iter().map(|(name, url)| serde_json::json!({"name": name, "url": url})).collect::<Vec<_>>(),
            "client_patches": {"enabled": client_patches},
            "telemetry": {"enabled": true, "opted_in": telemetry_opted_in, "prompt_answered": true, "auth_url": AUTH_URL},
            "unknown_future": "preserved verbatim",
        }))
        .unwrap();
        std::fs::write(launcher.join("launcher-config.json"), &config).unwrap();
        std::fs::write(
            launcher.join("install.json"),
            format!(
                r#"{{"schema_version":1,"install_id":"{LEGACY_INSTALL_ID}","machine_id":"fixture-machine","first_seen_ms":123456,"created_by_launcher_version":"0.8.4"}}"#
            ),
        )
        .unwrap();
        std::fs::write(
            game.join("launcher-installed.json"),
            br#"{"seed_sha256":"forged-current","seed_adopted":false,"applied_patches":[]}"#,
        )
        .unwrap();
        let source = migration::LegacySource {
            launcher_directory: launcher.canonicalize().unwrap(),
            game_directory: game.canonicalize().unwrap(),
        };
        Self {
            root,
            source,
            config,
            release: release(&bytes),
            seed,
        }
    }
    pub fn state_root(&self) -> PathBuf {
        self.root.path().canonicalize().unwrap().join("state")
    }
    pub fn destination(&self) -> PathBuf {
        self.root
            .path()
            .canonicalize()
            .unwrap()
            .join("desktop-copy")
    }
    /// Opens the private state and imports the legacy settings, as Settings does.
    pub fn open(&self) -> DesktopState {
        let mut state = DesktopState::open(&self.state_root()).unwrap();
        let preview = state.preview_legacy_import(&self.source).unwrap();
        state
            .import_legacy(&self.source, &preview.confirmation, 0)
            .unwrap();
        state
    }
    pub fn request(&self, state: &DesktopState) -> adoption::PreviewRequest {
        let (body, signature) = self.release.evidence();
        adoption::PreviewRequest {
            import_digest: state.legacy_import().unwrap().unwrap().confirmation,
            destination: self.destination(),
            operation_revision: state.operations().snapshot().revision,
            preferences_revision: state.preferences().revision,
            release: crate::catalog::verify_release(body, signature).unwrap(),
            artifacts: Some(adoption::Artifacts {
                seed: self.seed.clone(),
                patches: vec![],
            }),
        }
    }
    /// Every path and byte under both legacy folders.
    pub fn source_snapshot(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut found = Vec::new();
        for root in [&self.source.launcher_directory, &self.source.game_directory] {
            walk(root, &mut found);
        }
        found.sort();
        found
    }
}

fn walk(directory: &Path, found: &mut Vec<(PathBuf, Vec<u8>)>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.push((path.clone(), vec![]));
            walk(&path, found);
        } else {
            let bytes = std::fs::read(&path).unwrap();
            found.push((path, bytes));
        }
    }
}

/// Signed with the public development manifest key, as the other fixtures are.
fn release(seed: &[u8]) -> VerifiedRelease {
    let hash: String = Sha256::digest(seed)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"min_launcher":MINIMUM,
        "seed":{"blob":"seed.zip","size":seed.len(),"sha256":hash},"patches":[]}))
    .unwrap();
    let signature: String = SigningKey::from_bytes(&[0x2a; 32])
        .sign(&body)
        .to_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    crate::catalog::verify_release(&body, signature.as_bytes()).unwrap()
}

/// A launcher build older than every fixture release's signed minimum.
pub fn older_launcher() -> crate::launcher_compatibility::CompatibilityPolicy {
    use crate::launcher_compatibility::{CompatibilityPolicy, Identity};
    CompatibilityPolicy::new(
        Identity::from_parts(
            "0.1.0",
            None,
            Some("launcher-20261003-fixture"),
            Some("1791000000"),
        ),
        vec![],
    )
}

pub fn choices() -> adoption::Choices {
    adoption::Choices {
        normalize_managed_files: true,
        accept_unavailable_game_telemetry: true,
        old_game_closed: true,
    }
}

/// Native reference reconstruction: no helper, no Wine. The published copy
/// carries a Native backend, which macOS Play admission refuses by design.
pub fn adopt_native(legacy: &Legacy, state: &Arc<Mutex<DesktopState>>) -> adoption::Provenance {
    let request = legacy.request(&state.lock().unwrap());
    let preview = adoption::preview(
        state.clone(),
        request,
        CancellationToken::new(),
        ProgressSink::latest().0,
    )
    .unwrap();
    let handle = preview.report().preview_handle;
    adoption::confirm(
        preview,
        Uuid::new_v4(),
        handle,
        choices(),
        CancellationToken::new(),
    )
    .unwrap()
}

/// The pinned Windows archive helper and an optional verified runtime tree to
/// copy, so a supervised run needs no download. Both come from the environment.
pub struct Wine {
    helper: crate::mac_wine::HelperResource,
    runtime_tree: Option<PathBuf>,
}

impl Wine {
    pub fn from_environment() -> Self {
        let helper = crate::mac_wine::HelperResource::open(
            PathBuf::from(
                std::env::var_os("CIMMERIA_WINE_HELPER").expect("native-built pinned helper"),
            ),
            &std::env::var("CIMMERIA_WINE_HELPER_SHA256").expect("independent build identity"),
        )
        .unwrap();
        Self {
            helper,
            runtime_tree: std::env::var_os("CIMMERIA_WINE_RUNTIME_TREE").map(PathBuf::from),
        }
    }
}

/// The real Wine-backed adoption: the helper reconstructs the reference in this
/// fixture's own headless prefix, then the copy is confirmed and published.
pub async fn adopt_wine(
    legacy: &Legacy,
    state: &Arc<Mutex<DesktopState>>,
    wine: &Wine,
) -> adoption::Provenance {
    if let Some(tree) = &wine.runtime_tree {
        let runtimes = legacy.state_root().join("runtimes");
        std::fs::create_dir_all(&runtimes).unwrap();
        // Clone where the filesystem allows; cp falls back to a plain copy.
        let copied = std::process::Command::new("/bin/cp")
            .arg("-cR")
            .arg(tree)
            .arg(runtimes.join(tree.file_name().unwrap()))
            .status()
            .unwrap();
        assert!(copied.success(), "private runtime copy");
    }
    let request = legacy.request(&state.lock().unwrap());
    let mut worker =
        adoption::start_preview_wine(state.clone(), request, wine.helper.clone()).unwrap();
    let preview = worker.wait().await.unwrap();
    let handle = preview.report().preview_handle;
    let mut result = adoption::start_confirmation(preview, Uuid::new_v4(), handle, choices())
        .unwrap()
        .result;
    let published = result.wait_for(|value| value.is_some()).await.unwrap();
    published.clone().unwrap().unwrap()
}

/// Moves the private runtime copy aside, so a later prerequisite or Play worker
/// in the same test stops at its resource claim and never starts Wine.
pub fn retire_runtime(legacy: &Legacy) {
    let runtimes = legacy.state_root().join("runtimes");
    if runtimes.exists() {
        std::fs::rename(&runtimes, legacy.root.path().join("retired-runtimes")).unwrap();
    }
}

/// Successful prerequisite evidence for an installed Wine copy, recorded through
/// the public admission and observation entry points. No prerequisite process,
/// prefix or download: this is fixture evidence, not a prepared game runtime.
pub fn record_prepared_runtime(state: &mut DesktopState) -> Uuid {
    let installed = state.installed_content().unwrap().unwrap();
    let crate::ExtractionBackend::Wine { runtime_sha256, .. } = installed.intent.backend else {
        panic!("prerequisites apply to a Wine-backed copy");
    };
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_runtime_setup(
            Uuid::new_v4(),
            revision,
            installed.intent.operation_id,
            runtime_sha256,
            [9; 32],
        )
        .unwrap()
        .plan;
    state.begin_runtime_dispatch(plan.id).unwrap();
    state.record_runtime_host(plan.id, 123).unwrap();
    let mut report =
        cimmeria_runtime_probe::collect(cimmeria_runtime_probe::LoadResult::Loaded {}, |_| {
            cimmeria_runtime_probe::LoadResult::Loaded {}
        });
    report.physx_sdk = cimmeria_runtime_probe::physx::SdkResult::InitializedAndReleased {};
    state
        .record_runtime_observation(
            plan.id,
            cimmeria_runtime_probe::prerequisite::PrepareResult {
                schema_version: 1,
                operation_id: plan.id,
                prefix_generation: plan.prefix_generation,
                result: cimmeria_runtime_probe::prerequisite::ResultKind::Probed { report },
            },
        )
        .unwrap();
    state.finish_runtime_after_stop(plan.id).unwrap();
    installed.intent.operation_id
}
