//! Legacy schema-1 adoption fixtures, kept outside the production API.
use super::*;
use crate::install_worker::fixtures;
use serde_json::json;

// Legacy schema-1 copy fixtures exercise historic persisted journals. The public
// adoption API refuses Copy; only this private test path can construct one.
pub(super) fn preview(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    cancel: CancellationToken,
    progress: crate::install_progress::ProgressSink,
) -> Result<Preview, Error> {
    super::copy_preview(state, request, cancel, progress, Backend::Native, None)
}

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) state: Arc<Mutex<DesktopState>>,
    pub(super) source: migration::LegacySource,
    pub(super) seed: PathBuf,
    pub(super) release: VerifiedRelease,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let launcher = root.path().join("legacy");
        let game = root.path().join("old-game");
        std::fs::create_dir(&launcher).unwrap();
        std::fs::create_dir(&game).unwrap();
        let seed = root.path().join("seed.zip");
        let bytes = fixtures::archive(true);
        std::fs::write(&seed, &bytes).unwrap();
        let release = fixtures::verified(&bytes);
        let (progress, _) = crate::install_progress::ProgressSink::latest();
        crate::unpack::unpack(
            &seed,
            &game,
            &crate::unpack::UnpackSink {
                progress,
                label: "fixture".into(),
                cancel: CancellationToken::new(),
            },
        )
        .unwrap();
        crate::client_setup::prepare(
            &game,
            &[
                crate::client_setup::LoginServer {
                    name: "Second".into(),
                    url: "https://example.invalid/second".into(),
                },
                crate::client_setup::LoginServer {
                    name: "First".into(),
                    url: "https://example.invalid/first".into(),
                },
            ],
        )
        .unwrap();
        std::fs::write(launcher.join("launcher-config.json"), serde_json::to_vec_pretty(&json!({"install_path":"C:\\Old Game", "manifest_url":crate::catalog::URL,"login_servers":[{"name":"Second","url":"https://example.invalid/second"},{"name":"First","url":"https://example.invalid/first"}],"client_patches":{"enabled":false},"telemetry":{"enabled":true,"opted_in":true,"prompt_answered":true,"auth_url":"https://example.invalid/auth"},"unknown_future":"preserved verbatim"})).unwrap()).unwrap();
        std::fs::write(launcher.join("install.json"), br#"{"schema_version":1,"install_id":"72a8a13b-2a4e-4ea0-b5ba-5ba3cf0a619d","machine_id":"fixture-machine","first_seen_ms":123456,"created_by_launcher_version":"0.8.4"}"#).unwrap();
        std::fs::write(game.join("launcher-installed.json"), br#"{"seed_sha256":"forged-current","seed_adopted":false,"applied_patches":["duplicate","duplicate"]}"#).unwrap();
        let source = migration::LegacySource {
            launcher_directory: launcher.canonicalize().unwrap(),
            game_directory: game.canonicalize().unwrap(),
        };
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        let import = state.preview_legacy_import(&source).unwrap();
        state
            .import_legacy(&source, &import.confirmation, 0)
            .unwrap();
        Self {
            root,
            state: Arc::new(Mutex::new(state)),
            source,
            seed,
            release,
        }
    }
    pub(super) fn destination(&self) -> PathBuf {
        self.root
            .path()
            .canonicalize()
            .unwrap()
            .join("desktop-copy")
    }
    pub(super) fn request(&self) -> PreviewRequest {
        let state = self.state.lock().unwrap();
        PreviewRequest {
            source: SourceRequest::Legacy {
                import_digest: state.legacy_import().unwrap().unwrap().confirmation,
            },
            mode: AdoptionMode::Copy,
            destination: self.destination(),
            operation_revision: state.operations().snapshot().revision,
            preferences_revision: state.preferences().revision,
            release: crate::catalog::verify_release(
                self.release.evidence().0,
                self.release.evidence().1,
            )
            .unwrap(),
            artifacts: Some(Artifacts {
                seed: self.seed.clone(),
                patches: vec![],
            }),
        }
    }
    pub(super) fn preview(&self) -> Result<Preview, Error> {
        preview(
            self.state.clone(),
            self.request(),
            CancellationToken::new(),
            crate::install_progress::ProgressSink::latest().0,
        )
    }
    pub(super) fn source_snapshot(&self) -> (inventory::Index, inventory::Index) {
        (
            inventory::scan(&self.source.game_directory, &CancellationToken::new()).unwrap(),
            inventory::scan(&self.source.launcher_directory, &CancellationToken::new()).unwrap(),
        )
    }
}
pub(super) fn choices() -> Choices {
    Choices {
        normalize_managed_files: true,
        accept_unavailable_game_telemetry: true,
        old_game_closed: true,
        replace_modified_files: false,
    }
}
pub(super) fn commit(preview: Preview) -> Result<Provenance, Error> {
    let handle = preview.report.preview_handle;
    confirm(
        preview,
        Uuid::new_v4(),
        handle,
        choices(),
        CancellationToken::new(),
    )
}
