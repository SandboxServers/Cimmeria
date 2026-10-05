//! Retained fresh reconstruction. The result retains root/stage ownership for a
//! commit coordinator; no existing game is renamed by this layer.
use super::*;
use crate::{catalog, install, install_progress::ProgressSink, OperationState};
use futures_util::FutureExt;
use std::{io::Write, panic::AssertUnwindSafe, sync::Mutex, time::Duration};
use tokio::sync::{oneshot, watch};
use tokio_util::sync::CancellationToken;

pub struct Prepared {
    pub plan: Plan,
    pub(super) _root_owner: OwnerLock,
    pub(super) _work_owner: OwnerLock,
    #[cfg(target_os = "macos")]
    pub(super) wine: Option<crate::mac_wine::WineSeedExtractor>,
    // Dropping a delivered handoff notifies a retained observer without taking
    // the state mutex in Drop (the caller may already hold it).
    _handoff: Option<oneshot::Sender<()>>,
}
pub struct Preparation {
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    cancel: CancellationToken,
    pub progress: watch::Receiver<Option<install::Progress>>,
    pub result: oneshot::Receiver<Result<Prepared, Failure>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Cancelled,
    Failed,
    ReconciliationRequired,
}
impl Preparation {
    pub fn request_cancel(&self) -> Result<(), IntentError> {
        self.state
            .lock()
            .map_err(|_| StorageError::Io)?
            .operations_mut()?
            .request_cancel(self.id)?;
        self.cancel.cancel();
        Ok(())
    }
}
/// Native Windows backend only; macOS uses the explicit Wine entry point.
pub fn prepare_native(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
) -> Result<Preparation, IntentError> {
    if !cfg!(windows) {
        return Err(ContractError::InvalidTransition.into());
    }
    start(state, id, download_client()?, catalog::URL.into())
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
#[cfg(target_os = "macos")]
pub fn prepare_wine(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    helper: crate::mac_wine::HelperResource,
) -> Result<Preparation, IntentError> {
    start_with_backend(
        state,
        id,
        download_client()?,
        catalog::URL.into(),
        Backend::Wine(helper),
    )
}
enum Backend {
    Native,
    #[cfg(target_os = "macos")]
    Wine(crate::mac_wine::HelperResource),
}
struct Source {
    http: reqwest::Client,
    url: String,
    backend: Backend,
}
pub(crate) fn start(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    http: reqwest::Client,
    url: String,
) -> Result<Preparation, IntentError> {
    start_with_backend(state, id, http, url, Backend::Native)
}
fn start_with_backend(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    http: reqwest::Client,
    url: String,
    backend: Backend,
) -> Result<Preparation, IntentError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (plan, release) = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let plan = owner
            .repair_plan()?
            .ok_or(ContractError::UnknownOperation)?;
        if plan.id != id {
            return Err(ContractError::IdentityConflict.into());
        }
        match &backend {
            Backend::Native if !plan.installation.backend.is_native() => {
                return Err(ContractError::InvalidTransition.into())
            }
            Backend::Native => (),
            #[cfg(target_os = "macos")]
            Backend::Wine(helper) => {
                if helper.backend() != plan.installation.backend {
                    return Err(ContractError::IdentityConflict.into());
                }
                helper.verify().map_err(|_| StorageError::UnsafeFile)?;
            }
        }
        let installed = owner.installed_content()?.ok_or(StorageError::Corrupt)?;
        if installed.intent != plan.installation
            || installed.current_release != plan.release_identity()
        {
            return Err(StorageError::Corrupt.into());
        }
        owner
            .operations_mut()?
            .observe(id, OperationState::Running)?;
        (plan, installed.release)
    };
    let cancel = CancellationToken::new();
    let owned_cancel = cancel.clone();
    let (progress, observed) = ProgressSink::latest();
    let (send, result) = oneshot::channel();
    let owner = state.clone();
    runtime.spawn(async move {
        let result = AssertUnwindSafe(reconstruct(
            &owner,
            plan,
            release,
            Source { http, url, backend },
            owned_cancel,
            progress,
        ))
        .catch_unwind()
        .await
        .unwrap_or(Err(Failure::ReconciliationRequired));
        let result = result.map_err(|failure| finish_failure(&owner, id, failure));
        if send.send(result).is_err() {
            // Work finished, but its owner disappeared before accepting the commit
            // handoff. Preserve output; only explicit recovery may continue it.
            if let Ok(mut state) = owner.lock() {
                if let Ok(operations) = state.operations_mut() {
                    let _ = operations.mark_uncertain(id);
                }
            }
        }
    });
    Ok(Preparation {
        state,
        id,
        cancel,
        progress: observed,
        result,
    })
}
async fn reconstruct(
    state: &Arc<Mutex<DesktopState>>,
    plan: Plan,
    release: catalog::VerifiedRelease,
    source: Source,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Result<Prepared, Failure> {
    if cancel.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let candidate = plan.clone();
    let mut prepared = tokio::task::spawn_blocking(move || claim(candidate))
        .await
        .map_err(|_| Failure::ReconciliationRequired)?
        .map_err(|_| Failure::Failed)?;
    let stage = plan.stage();
    let context = install::InstallContext {
        manifest_url: &source.url,
        install_dir: &stage,
        manifest: release.manifest(),
        login_servers: &plan.installation.login_servers,
        cancel,
        progress,
        http: &source.http,
    };
    let (result, _) = match source.backend {
        Backend::Native => install::install_all(context).await,
        #[cfg(target_os = "macos")]
        Backend::Wine(helper) => {
            let adapter = crate::mac_wine::WineSeedExtractor::prepare(
                state.clone(),
                plan.id,
                helper.path().to_path_buf(),
                context.cancel.clone(),
                context.progress.clone(),
            )
            .await
            .map_err(|error| match error {
                crate::mac_wine::WineError::Runtime(
                    crate::mac_runtime::RuntimeError::Cancelled,
                ) => Failure::Cancelled,
                crate::mac_wine::WineError::Invalid => Failure::ReconciliationRequired,
                _ => Failure::Failed,
            })?;
            let cache = plan.work_directory().join("cache");
            let result = install::install_all_with_seed_extractor(
                context,
                install::SeedBackend {
                    extractor: &adapter,
                    cache_directory: &cache,
                },
            )
            .await;
            prepared.wine = Some(adapter);
            result
        }
    };
    match result {
        Err(install::InstallError::Cancelled) => return Err(Failure::Cancelled),
        Err(install::InstallError::SeedExtractionUncertain) => {
            return Err(Failure::ReconciliationRequired)
        }
        Err(_) => return Err(Failure::Failed),
        Ok(()) => (),
    }
    if !install_worker::content_valid(&stage, &release) {
        return Err(Failure::Failed);
    }
    let mut owner = state.lock().map_err(|_| Failure::ReconciliationRequired)?;
    if owner
        .repair_plan()
        .map_err(|_| Failure::ReconciliationRequired)?
        .as_ref()
        != Some(&plan)
    {
        return Err(Failure::ReconciliationRequired);
    }
    let result = atomic::write(
        &owner.directory.root,
        &format!("repair-prepared-{}.json", plan.id),
        &plan,
    );
    owner.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
    result.map_err(|_| Failure::ReconciliationRequired)?;
    drop(owner);
    let (handoff, abandoned) = oneshot::channel();
    let observed_state = state.clone();
    let id = plan.id;
    tokio::spawn(async move {
        if abandoned.await.is_err() {
            finish_failure(&observed_state, id, Failure::ReconciliationRequired);
        }
    });
    prepared._handoff = Some(handoff);
    Ok(prepared)
}
fn claim(plan: Plan) -> Result<Prepared, StorageError> {
    if plan
        .installation
        .destination
        .canonicalize()
        .map_err(|_| StorageError::InvalidDirectory)?
        != plan.installation.destination
    {
        return Err(StorageError::UnsafeFile);
    }
    let root_owner = lock_owner(&plan.installation)?;
    if directory_or_absent(&plan.installation.destination.join("game"))? != plan.original_present {
        return Err(StorageError::Corrupt);
    }
    match std::fs::symlink_metadata(plan.backup()) {
        Err(error) if error.kind() == ErrorKind::NotFound => (),
        Err(_) => return Err(StorageError::Io),
        Ok(_) => return Err(StorageError::UnsafeFile),
    }
    let work = plan.work_directory();
    std::fs::create_dir(&work).map_err(|_| StorageError::InUse)?;
    let work_owner = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(work.join("owner.json"))
        .map_err(|_| StorageError::Io)?;
    let mut work_owner = OwnerLock::acquire(work_owner).map_err(|_| StorageError::InUse)?;
    work_owner
        .write_all(&serde_json::to_vec(&plan).map_err(|_| StorageError::Corrupt)?)
        .map_err(|_| StorageError::Io)?;
    work_owner.sync_all().map_err(|_| StorageError::Io)?;
    #[cfg(unix)]
    for path in [&work, &plan.installation.destination] {
        File::open(path)
            .and_then(|f| f.sync_all())
            .map_err(|_| StorageError::Io)?;
    }
    Ok(Prepared {
        plan,
        _root_owner: root_owner,
        _work_owner: work_owner,
        #[cfg(target_os = "macos")]
        wine: None,
        _handoff: None,
    })
}
pub(super) fn finish_failure(state: &Mutex<DesktopState>, id: Uuid, failure: Failure) -> Failure {
    let Ok(mut owner) = state.lock() else {
        return Failure::ReconciliationRequired;
    };
    let Ok(operations) = owner.operations_mut() else {
        return Failure::ReconciliationRequired;
    };
    let result = match failure {
        Failure::Cancelled => operations.observe(id, OperationState::Cancelled),
        Failure::Failed => operations.observe(id, OperationState::Failed),
        Failure::ReconciliationRequired => operations.mark_uncertain(id),
    };
    if result.is_err() {
        let _ = operations.mark_uncertain(id);
        Failure::ReconciliationRequired
    } else {
        failure
    }
}
#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "macos"))]
mod wine_tests;
