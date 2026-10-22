//! Retained fresh reconstruction. The result retains root/stage ownership for a
//! commit coordinator; no existing game is renamed by this layer.
use super::*;
use crate::game_telemetry::install_event::{Ending, Run};
use crate::{catalog, install, install_progress::ProgressSink, OperationState};
use cimmeria_launcher_core::seed_source::SeedSourceHandle;
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
impl Failure {
    pub(crate) fn ending(self, code: &'static str) -> Ending {
        match self {
            Self::Cancelled => Ending::Cancelled,
            Self::Failed => Ending::Failed(code),
            Self::ReconciliationRequired => Ending::Uncertain,
        }
    }
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
    let (plan, release, path) = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let plan = owner
            .update_plan()?
            .ok_or(ContractError::UnknownOperation)?;
        if plan.id != id {
            return Err(ContractError::IdentityConflict.into());
        }
        match &backend {
            Backend::Native if !plan.owner.backend.is_native() => {
                return Err(ContractError::InvalidTransition.into())
            }
            Backend::Native => (),
            #[cfg(target_os = "macos")]
            Backend::Wine(helper) => {
                if helper.backend() != plan.owner.backend {
                    return Err(ContractError::IdentityConflict.into());
                }
                helper.verify().map_err(|_| StorageError::UnsafeFile)?;
            }
        }
        let installed = owner.installed_content()?.ok_or(StorageError::Corrupt)?;
        if installed.intent != plan.owner || installed.current_release != plan.previous {
            return Err(StorageError::Corrupt.into());
        }
        owner
            .operations_mut()?
            .observe(id, OperationState::Running)?;
        let release = owner
            .verify_release_identity(plan.target)
            .map_err(|_| StorageError::Corrupt)?;
        let path = choose_path(&mut owner, &plan, &release);
        (plan.clone(), release, path)
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
            path,
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
                    if let Err(error) = operations.mark_uncertain(id) {
                        events::mark_uncertain_failed(error);
                    }
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
/// How the replacement tree is built. Fixed under the state lock at admission.
enum UpdatePath {
    /// Today's rebuild from the seed (a fresh, downloaded or copy-adopted install).
    Full(SeedSourceHandle),
    /// Light-adopted install: seeded from the live tree, no seed (LA-13, D-LA14).
    Incremental(incremental::IncrementalPlan),
}

/// The seedless path needs the install's own adoption record to say `light`
/// (D-LA14) and then a ledger that matches the target. Anything else, or any
/// unreadable record, is the seed rebuild. A light install that is refused is
/// logged once, because it is the one that pays for a seed it never held.
fn choose_path(
    owner: &mut DesktopState,
    plan: &Plan,
    release: &catalog::VerifiedRelease,
) -> UpdatePath {
    let seedless = incremental::seedless_allowed(owner.adoption_check().ok().flatten());
    let incremental = if seedless {
        match owner.verify_release_identity(plan.previous).ok() {
            None => {
                events::seedless_declined("previous_release_unverified");
                None
            }
            Some(previous) => {
                let planned = incremental::plan(
                    &plan.owner.destination.join("game"),
                    &previous.manifest().patches,
                    &release.manifest().seed.sha256,
                    &release.manifest().patches,
                );
                if planned.is_none() {
                    events::seedless_declined("ledger_mismatch");
                }
                planned
            }
        }
    } else {
        None
    };
    match incremental {
        Some(incremental) => UpdatePath::Incremental(incremental),
        None => UpdatePath::Full(owner.seed_source_for(release)),
    }
}
async fn reconstruct(
    state: &Arc<Mutex<DesktopState>>,
    plan: Plan,
    release: catalog::VerifiedRelease,
    path: UpdatePath,
    source: Source,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Result<Prepared, Failure> {
    if cancel.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let started = std::time::Instant::now();
    let candidate = plan.clone();
    let mut prepared = tokio::task::spawn_blocking(move || claim(candidate))
        .await
        .map_err(|_| Failure::ReconciliationRequired)?
        .map_err(|_| Failure::Failed)?;
    let stage = plan.stage();
    let mut staging = events::Staging::FULL;
    let (seed_source, tolerant) = match path {
        UpdatePath::Full(seed_source) => (seed_source, false),
        UpdatePath::Incremental(incremental) => {
            let stats = incremental::stage(
                &plan,
                &incremental,
                &source.url,
                &source.http,
                &cancel,
                &progress,
            )
            .await?;
            staging = events::Staging {
                mode: "incremental",
                linked: stats.linked,
                copied: stats.copied,
                ..staging
            };
            // The stage already holds the install, so the seed is never read.
            (SeedSourceHandle::download(), true)
        }
    };
    let context = install::InstallContext {
        manifest_url: &source.url,
        install_dir: &stage,
        manifest: release.manifest(),
        login_servers: &plan.owner.login_servers,
        cancel: cancel.clone(),
        progress,
        http: &source.http,
        seed_source: &seed_source,
    };
    let (result, report) = match source.backend {
        Backend::Native if tolerant => install::install_all_tolerant(context).await,
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
            let backend = install::SeedBackend {
                extractor: &adapter,
                cache_directory: &cache,
            };
            let result = if tolerant {
                install::install_all_tolerant_with_seed_extractor(context, backend).await
            } else {
                install::install_all_with_seed_extractor(context, backend).await
            };
            prepared.wine = Some(adapter);
            result
        }
    };
    if let Ok(mut owner) = state.lock() {
        owner.stash_install_report(plan.id, report);
    }
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
    let scan_plan = plan.clone();
    let scan_cancel = cancel.clone();
    let digest = tokio::task::spawn_blocking(move || {
        staged_content::digest(&scan_plan.stage(), &scan_plan, &scan_cancel)
    })
    .await
    .map_err(|_| Failure::ReconciliationRequired)?
    .map_err(|_| {
        if cancel.is_cancelled() {
            Failure::Cancelled
        } else {
            Failure::Failed
        }
    })?;
    let size_root = stage.clone();
    let (files, bytes) = tokio::task::spawn_blocking(move || events::tree_size(&size_root, 0))
        .await
        .unwrap_or((0, 0));
    let mut owner = state.lock().map_err(|_| Failure::ReconciliationRequired)?;
    if owner
        .update_plan()
        .map_err(|_| Failure::ReconciliationRequired)?
        .as_ref()
        != Some(&plan)
    {
        return Err(Failure::ReconciliationRequired);
    }
    let result = atomic::write(
        &owner.directory.root,
        &format!("update-prepared-{}.json", plan.id),
        &staged_content::Ready {
            plan: plan.clone(),
            digest,
        },
    );
    owner.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
    result.map_err(|_| Failure::ReconciliationRequired)?;
    drop(owner);
    if tolerant {
        let skipped = crate::state::InstalledState::load(&stage).skipped_op_count();
        staging.ops_skipped = u64::try_from(skipped).unwrap_or(u64::MAX);
    }
    events::staged(files, bytes, started.elapsed(), staging);
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
        .owner
        .destination
        .canonicalize()
        .map_err(|_| StorageError::InvalidDirectory)?
        != plan.owner.destination
    {
        return Err(StorageError::UnsafeFile);
    }
    let root_owner = lock_owner(&plan.owner)?;
    if !directory_or_absent(&plan.owner.destination.join("game"))? {
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
    for path in [&work, &plan.owner.destination] {
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
    let open = operations.snapshot().operation.as_ref().is_some_and(|op| {
        op.id == id && !op.state.terminal() && op.state != OperationState::ReconciliationRequired
    });
    let result = match failure {
        Failure::Cancelled => operations.observe(id, OperationState::Cancelled),
        Failure::Failed => operations.observe(id, OperationState::Failed),
        Failure::ReconciliationRequired => operations.mark_uncertain(id),
    };
    let (finished, ended) = if result.is_err() {
        let ended = operations.mark_uncertain(id).is_ok();
        (Failure::ReconciliationRequired, ended)
    } else {
        (failure, true)
    };
    // Only the call that ended the run reports it: the dropped commit
    // handoff lands here after every commit, finding the run already over.
    if open && ended {
        owner.queue_install_result(id, Run::Update, finished.ending("update_failed"));
    }
    finished
}

#[cfg(all(test, target_os = "macos"))]
mod wine_tests;
