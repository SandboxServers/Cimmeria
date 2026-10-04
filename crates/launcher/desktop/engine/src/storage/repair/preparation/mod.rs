//! Retained fresh reconstruction. The result retains root/stage ownership for a
//! future commit coordinator; no existing game is renamed by this layer.
use super::*;
use crate::{catalog, install, install_progress::ProgressSink, OperationState};
use futures_util::FutureExt;
use std::{io::Write, panic::AssertUnwindSafe, sync::Mutex, time::Duration};
use tokio::sync::{oneshot, watch};
use tokio_util::sync::CancellationToken;

pub struct Prepared {
    pub plan: Plan,
    _root_owner: File,
    _work_owner: File,
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
/// Native Windows backend only. The Mac Wine adapter needs a separate work-ID
/// binding before it can use this preparation path.
pub fn prepare_native(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
) -> Result<Preparation, IntentError> {
    if !cfg!(windows) {
        return Err(ContractError::InvalidTransition.into());
    }
    let http = reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| StorageError::Io)?;
    start(state, id, http, catalog::URL.into())
}
fn start(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    http: reqwest::Client,
    url: String,
) -> Result<Preparation, IntentError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (plan, release) = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let plan = owner
            .repair_plan()?
            .ok_or(ContractError::UnknownOperation)?;
        if plan.id != id || !plan.installation.backend.is_native() {
            return Err(ContractError::IdentityConflict.into());
        }
        let installed = owner.installed_content()?.ok_or(StorageError::Corrupt)?;
        if installed.intent != plan.installation {
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
            http,
            url,
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
    state: &Mutex<DesktopState>,
    plan: Plan,
    release: catalog::VerifiedRelease,
    http: reqwest::Client,
    url: String,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Result<Prepared, Failure> {
    if cancel.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let candidate = plan.clone();
    let prepared = tokio::task::spawn_blocking(move || claim(candidate))
        .await
        .map_err(|_| Failure::ReconciliationRequired)?
        .map_err(|_| Failure::Failed)?;
    let stage = plan.stage();
    let (result, _) = install::install_all(install::InstallContext {
        manifest_url: &url,
        install_dir: &stage,
        manifest: release.manifest(),
        login_servers: &plan.installation.login_servers,
        cancel,
        progress,
        http: &http,
    })
    .await;
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
    let mut work_owner = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(work.join("owner.json"))
        .map_err(|_| StorageError::Io)?;
    work_owner.try_lock().map_err(|_| StorageError::InUse)?;
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
    })
}
fn finish_failure(state: &Mutex<DesktopState>, id: Uuid, failure: Failure) -> Failure {
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
