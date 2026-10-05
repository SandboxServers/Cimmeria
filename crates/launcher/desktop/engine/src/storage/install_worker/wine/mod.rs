//! Retained Mac install worker. Resource paths are supplied only by native code.
use super::*;
use crate::{
    mac_runtime::RuntimeError,
    mac_wine::{WineError, WineSeedExtractor},
};

/// Bind an admitted Wine intent to the packaged Windows helper. This native API
/// accepts no webview executable or environment. The shell must supply its
/// build-pinned resource identity during admission; a local file's hash alone is
/// not an artifact trust policy.
pub fn dispatch_wine(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    release: VerifiedRelease,
    helper: PathBuf,
) -> Result<Worker, IntentError> {
    dispatch_with(
        state,
        id,
        release,
        helper,
        catalog::URL.into(),
        download_client()?,
    )
}
fn dispatch_with(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    release: VerifiedRelease,
    helper: PathBuf,
    manifest_url: String,
    http: reqwest::Client,
) -> Result<Worker, IntentError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let input = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let intent = owner.install_intent()?.ok_or(StorageError::Corrupt)?;
        if intent.operation_id != id || intent.manifest_digest != release.digest() {
            return Err(ContractError::IdentityConflict.into());
        }
        crate::mac_wine::validate_helper(&intent, &helper).map_err(|_| StorageError::UnsafeFile)?;
        owner
            .operations_mut()?
            .observe(id, OperationState::Running)?;
        TaskInputs {
            intent,
            state_root: owner.directory.root.clone(),
            release,
            manifest_url,
            http,
            ownership: None,
            execution: Execution::Wine(helper),
        }
    };
    Ok(spawn_worker(runtime, state, id, input))
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn install(
    state: &Arc<Mutex<DesktopState>>,
    intent: &InstallIntent,
    state_root: &Path,
    release: &VerifiedRelease,
    manifest_url: &str,
    http: &reqwest::Client,
    helper: PathBuf,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Outcome {
    let Ok(_ownership) = claim(intent, state_root) else {
        return Outcome::DestinationUnavailable;
    };
    install_claimed(
        state,
        intent,
        release,
        manifest_url,
        http,
        helper,
        cancel,
        progress,
    )
    .await
}
#[allow(clippy::too_many_arguments)]
async fn install_claimed(
    state: &Arc<Mutex<DesktopState>>,
    intent: &InstallIntent,
    release: &VerifiedRelease,
    manifest_url: &str,
    http: &reqwest::Client,
    helper: PathBuf,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Outcome {
    let adapter = match WineSeedExtractor::prepare(
        state.clone(),
        intent.operation_id,
        helper,
        cancel.clone(),
        progress.clone(),
    )
    .await
    {
        Ok(adapter) => adapter,
        Err(error) => return preparation_outcome(error),
    };
    let stage = intent
        .destination
        .join(format!(".cimmeria-stage-{}", intent.operation_id));
    let cache = intent
        .destination
        .join(format!(".cimmeria-cache-{}", intent.operation_id));
    // Do not create staging here: the supervised helper requires an absent target.
    let (result, _) = install::install_all_with_seed_extractor(
        InstallContext {
            manifest_url,
            install_dir: &stage,
            manifest: release.manifest(),
            login_servers: &intent.login_servers,
            cancel,
            progress,
            http,
        },
        install::SeedBackend {
            extractor: &adapter,
            cache_directory: &cache,
        },
    )
    .await;
    finish_stage(intent, release, &stage, result)
}
fn preparation_outcome(error: WineError) -> Outcome {
    match error {
        WineError::RosettaRequired => Outcome::RosettaRequired,
        WineError::Runtime(RuntimeError::Cancelled) => Outcome::Cancelled,
        WineError::Runtime(_) => Outcome::RuntimeUnavailable,
        WineError::Invalid => Outcome::ReconciliationRequired,
    }
}
#[cfg(test)]
mod tests;
