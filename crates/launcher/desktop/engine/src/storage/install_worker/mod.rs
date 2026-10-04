//! Native-owned first-install task. Dropping a UI observer never aborts mutation.
use super::*;
use crate::{
    catalog::{self, VerifiedRelease},
    install::{self, InstallContext, InstallError},
    install_progress::ProgressSink,
    OperationState,
};
use futures_util::FutureExt;
use std::{io::Write, panic::AssertUnwindSafe, sync::Mutex, time::Duration};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Content preparation only; runtime prerequisites and game UAT are separate.
    ContentPrepared,
    Cancelled,
    DestinationUnavailable,
    InstallFailed,
    ContentInvalid,
    ReconciliationRequired,
}

pub struct Worker {
    id: Uuid,
    state: Arc<Mutex<DesktopState>>,
    cancel: CancellationToken,
    pub progress: watch::Receiver<Option<install::Progress>>,
    pub result: watch::Receiver<Option<Outcome>>,
}
impl Worker {
    pub fn operation_id(&self) -> Uuid {
        self.id
    }
    /// Commit cancellation before signalling the worker. No JoinHandle/abort API.
    pub fn request_cancel(&self) -> Result<Snapshot, IntentError> {
        let mut state = self.state.lock().map_err(|_| StorageError::Io)?;
        let snapshot = state.operations_mut()?.request_cancel(self.id)?;
        self.cancel.cancel();
        Ok(snapshot)
    }
}

/// Dispatch only a previously admitted, unstarted intent. Caller retains its
/// state owner; the task also retains it until completion, independently of views.
pub fn dispatch(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    release: VerifiedRelease,
) -> Result<Worker, IntentError> {
    dispatch_with(state, id, release, catalog::URL.into(), download_client()?)
}

fn download_client() -> Result<reqwest::Client, StorageError> {
    reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| StorageError::Io)
}

fn dispatch_with(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    release: VerifiedRelease,
    manifest_url: String,
    http: reqwest::Client,
) -> Result<Worker, IntentError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (intent, state_root) = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let intent = owner.install_intent()?.ok_or(StorageError::Corrupt)?;
        if intent.operation_id != id || intent.manifest_digest != release.digest() {
            return Err(ContractError::IdentityConflict.into());
        }
        if !intent.backend.is_native() {
            return Err(ContractError::InvalidTransition.into());
        }
        // Running is durable before claim/extraction. A second dispatch is rejected.
        owner
            .operations_mut()?
            .observe(id, OperationState::Running)?;
        (intent, owner.directory.root.clone())
    };
    Ok(spawn_worker(
        runtime,
        state,
        id,
        TaskInputs {
            intent,
            state_root,
            release,
            manifest_url,
            http,
            ownership: None,
        },
    ))
}

struct TaskInputs {
    intent: InstallIntent,
    state_root: PathBuf,
    release: VerifiedRelease,
    manifest_url: String,
    http: reqwest::Client,
    ownership: Option<File>,
}

fn spawn_worker(
    runtime: tokio::runtime::Handle,
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    input: TaskInputs,
) -> Worker {
    let cancel = CancellationToken::new();
    let (progress, observed) = ProgressSink::latest();
    let (result, results) = watch::channel(None);
    let owned_state = state.clone();
    let owned_cancel = cancel.clone();
    // Intentionally detached from the webview. No external owner can abort it.
    runtime.spawn(async move {
        let TaskInputs {
            intent,
            state_root,
            release,
            manifest_url,
            http,
            ownership,
        } = input;
        let outcome = AssertUnwindSafe(async {
            match ownership {
                Some(_guard) => {
                    install_stage(
                        &intent,
                        &release,
                        &manifest_url,
                        &http,
                        owned_cancel,
                        progress,
                    )
                    .await
                }
                None => {
                    install_owned(
                        &intent,
                        &state_root,
                        &release,
                        &manifest_url,
                        &http,
                        owned_cancel,
                        progress,
                    )
                    .await
                }
            }
        })
        .catch_unwind()
        .await
        .unwrap_or(Outcome::ReconciliationRequired);
        let published = publish(&owned_state, id, outcome);
        result.send_replace(Some(published));
    });
    Worker {
        id,
        state,
        cancel,
        progress: observed,
        result: results,
    }
}

fn publish(state: &Mutex<DesktopState>, id: Uuid, outcome: Outcome) -> Outcome {
    let Ok(mut state) = state.lock() else {
        return Outcome::ReconciliationRequired;
    };
    let Ok(operations) = state.operations_mut() else {
        return Outcome::ReconciliationRequired;
    };
    let result = match outcome {
        Outcome::ReconciliationRequired => operations.mark_uncertain(id),
        Outcome::ContentPrepared => operations.observe(id, OperationState::Succeeded),
        Outcome::Cancelled => operations.observe(id, OperationState::Cancelled),
        _ => operations.observe(id, OperationState::Failed),
    };
    if result.is_err() {
        // A failed terminal commit is never reported as a confirmed success.
        let _ = operations.mark_uncertain(id);
        Outcome::ReconciliationRequired
    } else {
        outcome
    }
}

async fn install_owned(
    intent: &InstallIntent,
    state_root: &Path,
    release: &VerifiedRelease,
    manifest_url: &str,
    http: &reqwest::Client,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Outcome {
    let Ok(_ownership) = claim(intent, state_root) else {
        return Outcome::DestinationUnavailable;
    };
    install_stage(intent, release, manifest_url, http, cancel, progress).await
}

async fn install_stage(
    intent: &InstallIntent,
    release: &VerifiedRelease,
    manifest_url: &str,
    http: &reqwest::Client,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Outcome {
    let stage = intent
        .destination
        .join(format!(".cimmeria-stage-{}", intent.operation_id));
    match std::fs::create_dir(&stage) {
        Ok(()) => (),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => (),
        Err(_) => return Outcome::DestinationUnavailable,
    }
    let (result, _) = install::install_all(InstallContext {
        manifest_url,
        install_dir: &stage,
        manifest: release.manifest(),
        login_servers: &intent.login_servers,
        cancel,
        progress,
        http,
    })
    .await;
    match result {
        Err(InstallError::Cancelled) => return Outcome::Cancelled,
        Err(InstallError::SeedExtractionUncertain) => return Outcome::ReconciliationRequired,
        Err(_) => return Outcome::InstallFailed,
        Ok(()) => (),
    }
    if !content_valid(&stage, release) {
        return Outcome::ContentInvalid;
    }
    promote(intent, &stage)
}

fn promote(intent: &InstallIntent, stage: &Path) -> Outcome {
    // Fresh owned layout only. Existing installations are never replaced here.
    let content = intent.destination.join("game");
    match std::fs::symlink_metadata(&content) {
        Err(error) if error.kind() == ErrorKind::NotFound => (),
        _ => return Outcome::DestinationUnavailable,
    }
    if std::fs::rename(stage, &content).is_err() {
        return Outcome::InstallFailed;
    }
    // Promotion is now visible: inability to persist its receipt is uncertain.
    if atomic::write(&intent.destination, "content-ready.json", intent).is_err()
        || sync_directory(&intent.destination).is_err()
    {
        return Outcome::ReconciliationRequired;
    }
    Outcome::ContentPrepared
}

fn claim(intent: &InstallIntent, state_root: &Path) -> Result<File, StorageError> {
    let root = super::install_intent::fresh_destination(&intent.destination, state_root)?;
    if root != intent.destination {
        return Err(StorageError::InvalidDirectory);
    }
    match std::fs::create_dir(&root) {
        Ok(()) => (),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => (),
        Err(_) => return Err(StorageError::Io),
    }
    let metadata = std::fs::symlink_metadata(&root).map_err(|_| StorageError::Io)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(StorageError::UnsafeFile);
    }
    // create_new arbitrates competing launchers even with different app-data roots.
    // Keep this marker on disk after close; existing ownership is never overwritten.
    let mut marker = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(root.join(".cimmeria-install.json"))
        .map_err(|_| StorageError::InUse)?;
    marker.try_lock().map_err(|_| StorageError::InUse)?;
    let bytes = serde_json::to_vec(intent).map_err(|_| StorageError::Corrupt)?;
    marker.write_all(&bytes).map_err(|_| StorageError::Io)?;
    marker.sync_all().map_err(|_| StorageError::Io)?;
    sync_directory(&root)?;
    sync_directory(root.parent().ok_or(StorageError::InvalidDirectory)?)?;
    Ok(marker)
}

fn sync_directory(path: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| StorageError::Io)?;
    #[cfg(not(unix))]
    let _ = path; // Windows directory power-loss durability remains a validation gate.
    Ok(())
}

pub(super) fn content_valid(stage: &Path, release: &VerifiedRelease) -> bool {
    let Ok(Some(state)) =
        read::<crate::state::InstalledState>(&crate::state::InstalledState::path(stage))
    else {
        return false;
    };
    let manifest = release.manifest();
    if state.seed_adopted
        || state.seed_sha256.as_deref() != Some(manifest.seed.sha256.as_str())
        || !state.missing_patches(&manifest.patches).is_empty()
    {
        return false;
    }
    let Ok(canonical_root) = stage.canonicalize() else {
        return false;
    };
    let exe = crate::install_layout::sgw_exe(stage);
    let game = crate::install_layout::sgwgame_dir(stage);
    let contained = |path: &Path| {
        path.canonicalize()
            .is_ok_and(|path| path.starts_with(&canonical_root))
    };
    std::fs::symlink_metadata(&exe)
        .is_ok_and(|meta| meta.is_file() && !meta.file_type().is_symlink() && meta.len() > 0)
        && contained(&exe)
        && game.is_dir()
        && contained(&game)
}

#[cfg(test)]
mod tests;

mod resume;
pub use resume::{resume, ResumeError};
