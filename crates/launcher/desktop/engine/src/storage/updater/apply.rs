//! Apply is a native capability; persisted paths are compared with the running app.
use super::{bundle, Config, DesktopState, Error, Phase, Record, Snapshot};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;
pub(super) mod process;
use process::{exchange, msiexec, rename_new, spawn, sync_parent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Target {
    Mac {
        bundle: PathBuf,
        executable: PathBuf,
        identifier: String,
    },
    Windows {
        executable: PathBuf,
    },
}
/// Constructed from std::env::current_exe(), never from an IPC request.
#[derive(Clone)]
pub struct InstalledTarget(Target);
impl InstalledTarget {
    pub fn current() -> Result<Self, Error> {
        let exe = std::env::current_exe().map_err(|_| Error::Target)?;
        Self::from_executable(&exe)
    }
    fn from_executable(exe: &Path) -> Result<Self, Error> {
        bundle::plain(exe)?;
        #[cfg(target_os = "macos")]
        {
            let bundle = exe
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent)
                .ok_or(Error::Target)?;
            if bundle.extension().and_then(|v| v.to_str()) != Some("app") {
                return Err(Error::Target);
            }
            let (identifier, _, relative) = bundle::identity(bundle)?;
            if bundle.join(&relative) != exe {
                return Err(Error::Target);
            }
            Ok(Self(Target::Mac {
                bundle: bundle.into(),
                executable: relative,
                identifier,
            }))
        }
        #[cfg(windows)]
        {
            Ok(Self(Target::Windows {
                executable: exe.into(),
            }))
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            Err(Error::Platform)
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Attempt {
    target: Target,
    original: String,
    replacement: Option<String>,
    staged_bundle: Option<String>,
    // Durable before any handoff; false means startup may safely restore old bytes.
    handoff: bool,
}
impl Attempt {
    fn paths(&self, owner: Uuid, state: &Path) -> Result<(PathBuf, PathBuf), Error> {
        match &self.target {
            Target::Mac { bundle, .. } => {
                let parent = bundle.parent().ok_or(Error::Target)?;
                bundle::plain(parent)?;
                Ok((
                    parent.join(format!(".cimmeria-update-{owner}")),
                    parent.join(format!(".cimmeria-backup-{owner}.app")),
                ))
            }
            Target::Windows { .. } => Ok((
                state.join(format!("launcher-update-{owner}.exe")),
                state.join(format!("launcher-update-{owner}.msi")),
            )),
        }
    }
}
impl DesktopState {
    pub fn apply_launcher_update(
        &mut self,
        config: Option<&Config>,
        target: &InstalledTarget,
        offer_id: Uuid,
        revision: u64,
        operation_revision: u64,
    ) -> Result<Snapshot, Error> {
        self.apply_launcher_update_with_handoff(
            config,
            target,
            offer_id,
            revision,
            operation_revision,
            || {},
        )
    }
    /// Notify the native owner after successful spawn, before fallible bookkeeping.
    /// The notification is not an installation acknowledgment or persisted intent.
    pub fn apply_launcher_update_with_handoff(
        &mut self,
        config: Option<&Config>,
        target: &InstalledTarget,
        offer_id: Uuid,
        revision: u64,
        operation_revision: u64,
        on_handoff: impl FnOnce(),
    ) -> Result<Snapshot, Error> {
        self.apply_update_with(
            config,
            target,
            offer_id,
            revision,
            operation_revision,
            notify_handoff(spawn, on_handoff),
        )
    }
    fn apply_update_with(
        &mut self,
        config: Option<&Config>,
        target: &InstalledTarget,
        offer_id: Uuid,
        revision: u64,
        operation_revision: u64,
        start: impl FnOnce(&Path, &[String]) -> Result<(), Error>,
    ) -> Result<Snapshot, Error> {
        let config = config.ok_or(Error::Disabled)?;
        let mut record = self.update_admission(revision, operation_revision)?;
        let offer = record
            .offer
            .as_ref()
            .filter(|o| o.id == offer_id)
            .ok_or(Error::StaleOffer)?;
        if record.phase != Phase::Ready {
            return Err(Error::StaleOffer);
        }
        // Read once into native memory and reverify the exact bytes passed to extraction.
        let bytes = self.staged_update_bytes()?;
        config.verify(offer, &bytes)?;
        match &target.0 {
            Target::Mac {
                bundle,
                identifier,
                executable,
            } => {
                if !config.platform.starts_with("darwin-") {
                    return Err(Error::Platform);
                }
                let (id, _, exe) = bundle::identity(bundle)?;
                if &id != identifier || &exe != executable {
                    return Err(Error::Target);
                }
            }
            Target::Windows { executable } => {
                if config.platform != "windows-x86_64" {
                    return Err(Error::Platform);
                }
                bundle::plain(executable)?;
            }
        }
        let original_path = match &target.0 {
            Target::Mac { bundle, .. } => bundle,
            Target::Windows { executable } => executable,
        };
        let attempt = Attempt {
            target: target.0.clone(),
            original: bundle::fingerprint(original_path)?,
            replacement: None,
            staged_bundle: None,
            handoff: false,
        };
        let owner = Uuid::new_v4();
        let (stage, backup) = attempt.paths(owner, self.state_root())?;
        if stage.try_exists().map_err(|_| Error::Target)?
            || backup.try_exists().map_err(|_| Error::Target)?
        {
            return Err(Error::Target);
        }
        record.owner = Some(owner);
        record.phase = Phase::Installing;
        record.failure = None;
        record.apply = Some(attempt);
        record = self.save_update(record)?;
        let result = self.replace_update(&mut record, &bytes, &stage, &backup, start);
        if let Err(error) = result {
            if self.requires_reopen() {
                return Err(error);
            }
            // Roll back only a fully recognized transaction. Unknown state keeps ownership.
            if (!record.apply.as_ref().is_some_and(|a| a.handoff) || error == Error::Spawn)
                && self.restore_update(&record).is_ok()
            {
                record.phase = Phase::Failed;
                record.owner = None;
                record.failure = Some(error);
            } else {
                record.phase = Phase::ReconciliationRequired;
                record.failure = Some(Error::Reconciliation);
            }
            self.save_update(record)?;
            return Err(error);
        }
        self.launcher_update_snapshot(Some(config))
    }
    fn replace_update(
        &mut self,
        record: &mut Record,
        bytes: &[u8],
        stage: &Path,
        backup: &Path,
        start: impl FnOnce(&Path, &[String]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let attempt = record.apply.as_ref().ok_or(Error::Reconciliation)?.clone();
        match &attempt.target {
            Target::Mac {
                bundle,
                executable,
                identifier,
            } => {
                let extracted =
                    bundle::extract(bytes, stage, record.owner.ok_or(Error::Reconciliation)?)?;
                let (id, version, exe) = bundle::identity(&extracted)?;
                let offered = record
                    .offer
                    .as_ref()
                    .ok_or(Error::StaleOffer)?
                    .version
                    .trim_start_matches('v');
                if &id != identifier || &exe != executable || version != offered {
                    return Err(Error::Package);
                }
                let a = record.apply.as_mut().unwrap();
                a.replacement = Some(bundle::fingerprint(&extracted)?);
                a.staged_bundle = Some(
                    extracted
                        .file_name()
                        .ok_or(Error::Package)?
                        .to_str()
                        .ok_or(Error::Package)?
                        .into(),
                );
                *record = self.save_update(record.clone())?;
                // Both paths are siblings on the target volume; no copy fallback or privilege escalation.
                if bundle::fingerprint(bundle)? != attempt.original {
                    return Err(Error::Target);
                }
                // Atomic exchange keeps the installed path launchable even if
                // the process dies before the original reaches its backup name.
                exchange(bundle, &extracted)?;
                sync_parent(bundle)?;
                *record = self.save_update(record.clone())?;
                rename_new(&extracted, backup)?;
                sync_parent(bundle)?;
                *record = self.save_update(record.clone())?;
                // Intent is durable before spawn. Recovery never repeats an ambiguous spawn.
                record.apply.as_mut().unwrap().handoff = true;
                *record = self.save_update(record.clone())?;
                start(
                    &bundle.join(executable),
                    &["--launcher-update-restart".into()],
                )?;
            }
            Target::Windows { executable } => {
                let offer = record.offer.as_ref().ok_or(Error::StaleOffer)?;
                let url = reqwest::Url::parse(&offer.url).map_err(|_| Error::Package)?;
                let msi = url.path().ends_with(".msi");
                if !msi && !url.path().ends_with(".exe") {
                    return Err(Error::Package);
                }
                let installer = if msi { backup } else { stage };
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(installer)
                    .map_err(|_| Error::Replace)?;
                use std::io::Write;
                file.write_all(bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|_| Error::Replace)?;
                drop(file);
                record.apply.as_mut().unwrap().replacement = Some(bundle::fingerprint(installer)?);
                record.apply.as_mut().unwrap().staged_bundle =
                    Some(if msi { "msi" } else { "exe" }.into());
                record.apply.as_mut().unwrap().handoff = true;
                *record = self.save_update(record.clone())?;
                if msi {
                    start(
                        &msiexec()?,
                        &[
                            "/i".into(),
                            installer.to_string_lossy().into_owned(),
                            "/passive".into(),
                            "/norestart".into(),
                            "AUTOLAUNCHAPP=True".into(),
                            "LAUNCHAPPARGS=--launcher-update-restart".into(),
                        ],
                    )?;
                } else {
                    start(
                        installer,
                        &[
                            "/P".into(),
                            "/R".into(),
                            "/UPDATE".into(),
                            format!("/D={}", executable.parent().ok_or(Error::Target)?.display()),
                        ],
                    )?;
                }
            }
        }
        record.phase = Phase::RestartRequired;
        record.failure = None;
        *record = self.save_update(record.clone())?;
        Ok(())
    }
    /// Called only during native startup, before exposing game mutation commands.
    /// Compiled identity is evidence of startup, not a claim of application health.
    pub fn reconcile_launcher_update(
        &mut self,
        target: &InstalledTarget,
        compiled_version: &str,
    ) -> Result<(), Error> {
        let mut record = self.update_record()?;
        if !matches!(
            record.phase,
            Phase::ReconciliationRequired | Phase::RestartRequired | Phase::Installing
        ) {
            return Ok(());
        }
        let attempt = record.apply.as_ref().ok_or(Error::Reconciliation)?;
        if attempt.target != target.0 {
            return Err(Error::Target);
        }
        let expected = record
            .offer
            .as_ref()
            .ok_or(Error::StaleOffer)?
            .version
            .trim_start_matches('v');
        if compiled_version == expected {
            if let Target::Mac { bundle, .. } = &attempt.target {
                if attempt.replacement.as_deref() != Some(bundle::fingerprint(bundle)?.as_str()) {
                    return Err(Error::Reconciliation);
                }
            }
            let cleanup = record.clone();
            record.phase = Phase::Installed;
            record.owner = None;
            record.failure = None;
            self.save_update(record)?;
            // Startup is acknowledged before best-effort garbage collection. A crash
            // during deletion cannot turn a partially deleted backup into false failure.
            let _ = self.cleanup_update(&cleanup);
        } else if !attempt.handoff {
            self.restore_update(&record)?;
            record.phase = Phase::Failed;
            record.owner = None;
            record.failure = Some(Error::Interrupted);
            self.save_update(record)?;
        }
        // A handed-off Windows installer may still be running; an older process
        // reopening cannot infer cancellation and must retain reconciliation ownership.
        Ok(())
    }
    fn restore_update(&self, record: &Record) -> Result<(), Error> {
        let attempt = record.apply.as_ref().ok_or(Error::Reconciliation)?;
        let (stage, backup) = attempt.paths(
            record.owner.ok_or(Error::Reconciliation)?,
            self.state_root(),
        )?;
        if let Target::Mac { bundle, .. } = &attempt.target {
            if stage.exists() {
                bundle::owned_stage(&stage, record.owner.ok_or(Error::Reconciliation)?)?;
            }
            let installed = bundle::fingerprint(bundle).ok();
            if backup.exists() {
                let saved = bundle::fingerprint(&backup)?;
                if saved == attempt.original {
                    if attempt.replacement.is_some()
                        && installed.as_deref() == attempt.replacement.as_deref()
                    {
                        exchange(bundle, &backup)?;
                        sync_parent(bundle)?;
                        // After exchange the backup contains only recognized new bytes.
                        fs::remove_dir_all(&backup).map_err(|_| Error::Replace)?;
                    } else if installed.is_none() {
                        // Recovery for interrupted attempts written by older builds.
                        rename_new(&backup, bundle)?;
                        sync_parent(bundle)?;
                    } else {
                        return Err(Error::Reconciliation);
                    }
                } else if installed.as_deref() == Some(attempt.original.as_str())
                    && Some(saved.as_str()) == attempt.replacement.as_deref()
                {
                    fs::remove_dir_all(&backup).map_err(|_| Error::Replace)?;
                } else {
                    return Err(Error::Reconciliation);
                }
            } else if installed.as_deref() != Some(attempt.original.as_str()) {
                let original = stage.join(
                    attempt
                        .staged_bundle
                        .as_ref()
                        .ok_or(Error::Reconciliation)?,
                );
                if installed.as_deref() != attempt.replacement.as_deref()
                    || bundle::fingerprint(&original)? != attempt.original
                {
                    return Err(Error::Reconciliation);
                }
                exchange(bundle, &original)?;
                sync_parent(bundle)?;
            }
            if stage.exists() {
                bundle::owned_stage(&stage, record.owner.ok_or(Error::Reconciliation)?)?;
                fs::remove_dir_all(stage).map_err(|_| Error::Replace)?;
            }
        } else {
            // Called for a synchronous spawn failure, never for uncertain successful handoff.
            for path in [stage, backup] {
                if path.exists() {
                    if attempt.replacement.as_deref() != Some(bundle::fingerprint(&path)?.as_str())
                    {
                        return Err(Error::Reconciliation);
                    }
                    bundle::plain(&path)?;
                    fs::remove_file(path).map_err(|_| Error::Replace)?;
                }
            }
        }
        Ok(())
    }
    fn cleanup_update(&self, record: &Record) -> Result<(), Error> {
        let attempt = record.apply.as_ref().ok_or(Error::Reconciliation)?;
        let (stage, backup) = attempt.paths(
            record.owner.ok_or(Error::Reconciliation)?,
            self.state_root(),
        )?;
        match &attempt.target {
            Target::Mac { .. } => {
                if stage.exists() {
                    bundle::owned_stage(&stage, record.owner.ok_or(Error::Reconciliation)?)?;
                }
                if let Some(name) = &attempt.staged_bundle {
                    let held = stage.join(name);
                    if held.exists() {
                        let hash = bundle::fingerprint(&held)?;
                        if hash != attempt.original
                            && Some(hash.as_str()) != attempt.replacement.as_deref()
                        {
                            return Err(Error::Reconciliation);
                        }
                    }
                }
                if backup.exists() {
                    if bundle::fingerprint(&backup)? != attempt.original {
                        return Err(Error::Reconciliation);
                    }
                    bundle::plain(&backup)?;
                    fs::remove_dir_all(&backup).map_err(|_| Error::Replace)?;
                }
                if stage.exists() {
                    bundle::owned_stage(&stage, record.owner.ok_or(Error::Reconciliation)?)?;
                    fs::remove_dir_all(stage).map_err(|_| Error::Replace)?;
                }
            }
            Target::Windows { .. } => {
                for path in [stage, backup] {
                    if path.exists() {
                        if attempt.replacement.as_deref()
                            != Some(bundle::fingerprint(&path)?.as_str())
                        {
                            return Err(Error::Reconciliation);
                        }
                        bundle::plain(&path)?;
                        fs::remove_file(path).map_err(|_| Error::Replace)?;
                    }
                }
            }
        }
        Ok(())
    }
}
// Only a successful native spawn emits this signal. Durable intent precedes
// spawn and therefore cannot be used as evidence that shutdown is required.
fn notify_handoff(
    start: impl FnOnce(&Path, &[String]) -> Result<(), Error>,
    on_handoff: impl FnOnce(),
) -> impl FnOnce(&Path, &[String]) -> Result<(), Error> {
    move |path, args| {
        start(path, args)?;
        on_handoff();
        Ok(())
    }
}

#[cfg(feature = "test-support")]
impl DesktopState {
    /// Inert Windows installer seam for engine-to-host lifecycle fault tests.
    pub fn apply_launcher_update_fixture(
        &mut self,
        config: &Config,
        executable: &Path,
        ready: Snapshot,
        start: impl FnOnce(&Path, &[String]) -> Result<(), Error>,
        on_handoff: impl FnOnce(),
    ) -> Result<Snapshot, Error> {
        self.apply_update_with(
            Some(config),
            &InstalledTarget(Target::Windows {
                executable: executable.into(),
            }),
            ready.offer.ok_or(Error::StaleOffer)?.id,
            ready.revision,
            ready.operation_revision,
            notify_handoff(start, on_handoff),
        )
    }
}

#[cfg(test)]
mod tests;
