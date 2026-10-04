//! Headless seed extraction through an independently owned Wine prefix.
//! This environment deliberately cannot display game windows. Game launch needs
//! a separate graphics/prerequisite policy; this adapter never claims Play-ready.
use crate::{
    archive_worker::ExtractRequest,
    helper_supervisor::{self, Deadlines, HelperCommand, Outcome},
    install::{InstallError, SeedExtraction, SeedExtractor},
    install_progress::{ProgressReporter, ProgressSink},
    mac_runtime, DesktopState, ExtractionBackend, InstallIntent,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{File, OpenOptions},
    future::Future,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
mod paths;
#[derive(Debug, thiserror::Error)]
pub enum WineError {
    #[error("Wine adapter ownership or resource validation failed")]
    Invalid,
    #[error("Rosetta must be installed before using this Wine runtime")]
    RosettaRequired,
    #[error("Wine runtime preparation failed")]
    Runtime(#[from] mac_runtime::RuntimeError),
}
pub struct WineSeedExtractor {
    state: Arc<Mutex<DesktopState>>,
    intent: InstallIntent,
    runtime: PathBuf,
    helper: PathBuf,
    prefix: PathBuf,
    _owner: File,
    used: std::sync::atomic::AtomicBool,
    limits: Deadlines,
}
impl WineSeedExtractor {
    /// Helper path is a native-selected bundled resource. Expected identities come
    /// from durable intent; no webview executable, environment or path is accepted.
    pub async fn prepare(
        state: Arc<Mutex<DesktopState>>,
        id: Uuid,
        helper: PathBuf,
        cancel: CancellationToken,
        progress: ProgressSink,
    ) -> Result<Self, WineError> {
        let (intent, state_root) = {
            let owner = state.lock().map_err(|_| WineError::Invalid)?;
            if owner.requires_reopen() {
                return Err(WineError::Invalid);
            }
            let intent = owner
                .install_intent()
                .map_err(|_| WineError::Invalid)?
                .ok_or(WineError::Invalid)?;
            if intent.operation_id != id {
                return Err(WineError::Invalid);
            }
            (intent, owner.state_root().to_path_buf())
        };
        let ExtractionBackend::Wine {
            runtime_sha256,
            helper_sha256,
        } = &intent.backend
        else {
            return Err(WineError::Invalid);
        };
        if hex(runtime_sha256) != mac_runtime::ARCHIVE_SHA256 {
            return Err(WineError::Invalid);
        }
        let candidate = helper.clone();
        let expected = *helper_sha256;
        tokio::task::spawn_blocking(move || verify_file(&candidate, &expected))
            .await
            .map_err(|_| WineError::Invalid)??;
        let rosetta = tokio::process::Command::new("/usr/bin/arch")
            .args(["-x86_64", "/usr/bin/true"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .status();
        if !tokio::time::timeout(std::time::Duration::from_secs(5), rosetta)
            .await
            .map_err(|_| WineError::RosettaRequired)?
            .map_err(|_| WineError::RosettaRequired)?
            .success()
        {
            return Err(WineError::RosettaRequired);
        }
        let runtime =
            mac_runtime::prepare(state_root.join("runtimes"), cancel.clone(), progress).await?;
        if cancel.is_cancelled() {
            return Err(mac_runtime::RuntimeError::Cancelled.into());
        }
        let prefix_root = state_root.join("wine-prefixes");
        let evidence = intent.clone();
        let (prefix, owner) =
            tokio::task::spawn_blocking(move || claim_prefix(&prefix_root, &evidence))
                .await
                .map_err(|_| WineError::Invalid)??;
        Ok(Self {
            state,
            intent,
            runtime,
            helper,
            prefix,
            _owner: owner,
            used: std::sync::atomic::AtomicBool::new(false),
            limits: Deadlines::default(),
        })
    }
    async fn stop_prefix(&self) -> Result<(), WineError> {
        let environment = self.command()?.environment;
        // This prefix is exclusive to the extraction attempt, never a user's
        // existing profile or a running game. -w confirms server lock release.
        for argument in ["-k", "-w"] {
            let result = tokio::process::Command::new(self.runtime.join("bin/wineserver"))
                .arg(argument)
                .env_clear()
                .envs(&environment)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .status();
            let status = tokio::time::timeout(std::time::Duration::from_secs(10), result)
                .await
                .map_err(|_| WineError::Invalid)?
                .map_err(|_| WineError::Invalid)?;
            // A server may already be absent when -k runs; -w is authoritative.
            if argument == "-w" && !status.success() {
                return Err(WineError::Invalid);
            }
        }
        Ok(())
    }
    fn command(&self) -> Result<HelperCommand, WineError> {
        let mut environment = BTreeMap::<OsString, OsString>::new();
        for (key, value) in [
            ("WINEPREFIX", self.prefix.clone().into_os_string()),
            (
                "WINESERVER",
                self.runtime.join("bin/wineserver").into_os_string(),
            ),
            (
                "DYLD_LIBRARY_PATH",
                self.runtime.join("lib/external").into_os_string(),
            ),
            (
                "HOME",
                self.prefix
                    .parent()
                    .ok_or(WineError::Invalid)?
                    .as_os_str()
                    .to_owned(),
            ),
            ("PATH", "/usr/bin:/bin".into()),
            ("WINEDEBUG", "-all".into()),
            (
                "WINEDLLOVERRIDES",
                "winemac.drv,winex11.drv,winewayland.drv,winemenubuilder.exe,mscoree,mshtml=d"
                    .into(),
            ),
        ] {
            environment.insert(key.into(), value);
        }
        Ok(HelperCommand {
            executable: self.runtime.join("bin/wine"),
            arguments: vec![paths::guest(&self.helper)?.into()],
            directory: self
                .prefix
                .parent()
                .ok_or(WineError::Invalid)?
                .to_path_buf(),
            environment,
        })
    }
}
impl SeedExtractor for WineSeedExtractor {
    fn extract<'a>(
        &'a self,
        request: SeedExtraction<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), InstallError>> + Send + 'a>> {
        Box::pin(async move {
            let uncertain = || InstallError::SeedExtractionUncertain;
            let stage = self
                .intent
                .destination
                .join(format!(".cimmeria-stage-{}", self.intent.operation_id));
            let cache = self
                .intent
                .destination
                .join(format!(".cimmeria-cache-{}", self.intent.operation_id));
            if request.destination != stage
                || request.archive.parent() != Some(cache.as_path())
                || std::fs::symlink_metadata(&stage).is_ok()
            {
                return Err(uncertain());
            }
            if self.used.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return Err(uncertain());
            }
            let guest = ExtractRequest {
                schema_version: 1,
                operation_id: self.intent.operation_id,
                archive: paths::guest(request.archive)
                    .map_err(|_| uncertain())?
                    .into(),
                destination: paths::guest(request.destination)
                    .map_err(|_| uncertain())?
                    .into(),
                sha256: request.sha256.into(),
            };
            let (progress, mut updates) = tokio::sync::watch::channel(None);
            let running = helper_supervisor::run_owned(
                self.state.clone(),
                self.command().map_err(|_| uncertain())?,
                guest,
                request.cancel,
                progress,
                self.limits,
            );
            tokio::pin!(running);
            let outcome = loop {
                tokio::select! {
                    result=&mut running=>break result,
                    result=updates.changed()=>{if result.is_err(){break running.await;}
                        if let Some(value)=*updates.borrow_and_update(){request.progress.report(crate::install::Progress::Extracting{
                            label:"seed".into(),current:value.current as usize,total:value.total as usize,filename:String::new()});}
                    }
                }
            };
            self.stop_prefix().await.map_err(|_| uncertain())?;
            let outcome = outcome.map_err(|_| uncertain())?;
            match outcome {
                Outcome::Completed => Ok(()),
                Outcome::Cancelled => Err(InstallError::Cancelled),
                Outcome::ReconciliationRequired(_) => Err(uncertain()),
                _ => Err(InstallError::Io(std::io::Error::other(
                    "Wine seed extraction failed",
                ))),
            }
        })
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn verify_file(path: &Path, expected: &[u8; 32]) -> Result<(), WineError> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| WineError::Invalid)?;
    if !path.is_absolute() || !meta.is_file() || meta.len() > 128 * 1024 * 1024 {
        return Err(WineError::Invalid);
    }
    let mut file = File::open(path).map_err(|_| WineError::Invalid)?;
    let mut hash = Sha256::new();
    let mut bytes = [0u8; 65536];
    loop {
        let n = file.read(&mut bytes).map_err(|_| WineError::Invalid)?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    if <[u8; 32]>::from(hash.finalize()) != *expected {
        return Err(WineError::Invalid);
    }
    Ok(())
}
fn claim_prefix(root: &Path, intent: &InstallIntent) -> Result<(PathBuf, File), WineError> {
    let io = |_| WineError::Invalid;
    std::fs::create_dir_all(root).map_err(io)?;
    if root.canonicalize().map_err(io)? != root {
        return Err(WineError::Invalid);
    }
    let owned = root.join(intent.operation_id.to_string());
    // Existing prefix owners cannot silently be adopted or replayed after restart.
    std::fs::create_dir(&owned).map_err(io)?;
    let mut marker = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(owned.join("owner.json"))
        .map_err(io)?;
    marker.try_lock().map_err(|_| WineError::Invalid)?;
    marker
        .write_all(&serde_json::to_vec(intent).map_err(|_| WineError::Invalid)?)
        .map_err(io)?;
    marker.sync_all().map_err(io)?;
    let prefix = owned.join("bottle");
    std::fs::create_dir(&prefix).map_err(io)?;
    std::fs::create_dir(prefix.join("drive_c")).map_err(io)?;
    std::fs::create_dir(prefix.join("dosdevices")).map_err(io)?;
    // Creating dosdevices ourselves suppresses Wine's default drive setup.
    // Supply C: as well as Z: before first boot can populate Windows files.
    std::os::unix::fs::symlink("../drive_c", prefix.join("dosdevices/c:")).map_err(io)?;
    std::os::unix::fs::symlink("/", prefix.join("dosdevices/z:")).map_err(io)?;
    File::open(&owned).map_err(io)?.sync_all().map_err(io)?;
    File::open(root).map_err(io)?.sync_all().map_err(io)?;
    Ok((prefix, marker))
}
#[cfg(test)]
mod tests;
