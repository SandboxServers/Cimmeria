//! Apply is a native capability; persisted paths are compared with the running app.
use super::{bundle, Config, DesktopState, Error, Phase, Record, Snapshot};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;
pub(super) mod process;
mod zip_names;
pub(super) mod zip_swap;
use process::{exchange, rename_new, spawn, sync_parent};

/// Fixed restart argument; the old process id follows it so the new one can
/// wait for this process to exit before taking the single-instance guard.
pub const RESTART_ARG: &str = "--launcher-update-restart";
fn restart_args() -> Vec<String> {
    vec![RESTART_ARG.into(), std::process::id().to_string()]
}
/// D-LX3: the Windows package is the release zip, never an installer.
pub(super) fn require_zip_url(url: &str) -> Result<(), Error> {
    let url = reqwest::Url::parse(url).map_err(|_| Error::Package)?;
    if !url.path().to_ascii_lowercase().ends_with(".zip") {
        tracing::warn!(target: "launcher.update", event = "apply_refused", reason = "not_a_zip", "Windows launcher update package is not a zip");
        return Err(Error::Package);
    }
    Ok(())
}

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
    /// Windows zip swap, saved before the first rename.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    swap: Option<zip_swap::Plan>,
}
impl Attempt {
    fn paths(&self, owner: Uuid, _state: &Path) -> Result<(PathBuf, PathBuf), Error> {
        match &self.target {
            Target::Mac { bundle, .. } => {
                let parent = bundle.parent().ok_or(Error::Target)?;
                bundle::plain(parent)?;
                Ok((
                    parent.join(format!(".cimmeria-update-{owner}")),
                    parent.join(format!(".cimmeria-backup-{owner}.app")),
                ))
            }
            Target::Windows { executable } => {
                // The stage sits beside the executable so every swap rename stays
                // on one volume. Windows has no separate backup path: replaced
                // files wait beside their originals as `<name>.old`.
                let stage = executable.parent().ok_or(Error::Target)?.join(
                    self.staged_bundle
                        .clone()
                        .unwrap_or_else(|| format!(".update-{owner}")),
                );
                Ok((stage.clone(), stage))
            }
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
                require_zip_url(&offer.url)?;
            }
        }
        let original_path = match &target.0 {
            Target::Mac { bundle, .. } => bundle,
            Target::Windows { executable } => executable,
        };
        let owner = Uuid::new_v4();
        let attempt = Attempt {
            target: target.0.clone(),
            original: bundle::fingerprint(original_path)?,
            replacement: None,
            // Known before any write, so recovery finds a partial Windows stage.
            // The owner in the name keeps a retry clear of an earlier stage.
            staged_bundle: match &target.0 {
                Target::Mac { .. } => None,
                Target::Windows { .. } => Some(zip_swap::stage_name(
                    offer.version.trim_start_matches('v'),
                    owner,
                )),
            },
            handoff: false,
            swap: None,
        };
        if let Target::Windows { executable } = &target.0 {
            // Phase is Ready, so no attempt is in flight: every marked stage
            // here is an earlier attempt's leftover.
            zip_swap::sweep_stale_stages(executable.parent().ok_or(Error::Target)?, Some(owner));
        }
        let (stage, backup) = attempt.paths(owner, self.state_root())?;
        if stage.try_exists().map_err(|_| Error::Target)?
            || backup.try_exists().map_err(|_| Error::Target)?
        {
            tracing::warn!(target: "launcher.update", event = "apply_refused", reason = "stage_collision", stage = %stage.display(), "update stage path already exists; it is never adopted");
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
                tracing::warn!(target: "launcher.update", event = "apply_failed", ?error, rolled_back = true, "launcher update failed; the installed launcher was restored");
                record.phase = Phase::Failed;
                record.owner = None;
                record.failure = Some(error);
            } else {
                tracing::error!(target: "launcher.update", event = "apply_failed", ?error, rolled_back = false, "launcher update failed and was not rolled back; reconciliation required");
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
                start(&bundle.join(executable), &restart_args())?;
            }
            Target::Windows { executable } => {
                let _ = backup;
                let install = executable.parent().ok_or(Error::Target)?;
                let owner = record.owner.ok_or(Error::Reconciliation)?;
                let staged = zip_swap::extract(bytes, stage, owner)?;
                let plan = zip_swap::plan(install, executable, owner, staged)?;
                let new_exe = zip_swap::executable_source(&plan).ok_or(Error::Package)?;
                let a = record.apply.as_mut().unwrap();
                a.replacement = Some(bundle::fingerprint(&stage.join(new_exe))?);
                a.swap = Some(plan.clone());
                // The plan is durable before the first rename, so a crash at any
                // point is rolled back from the record, never guessed.
                *record = self.save_update(record.clone())?;
                if bundle::fingerprint(executable)? != attempt.original {
                    return Err(Error::Target);
                }
                zip_swap::swap_in(install, stage, &plan)?;
                record.apply.as_mut().unwrap().handoff = true;
                *record = self.save_update(record.clone())?;
                // CreateProcess, not ShellExecute: the launcher relaunches itself,
                // needs no elevation and must not block on a shell error dialog.
                start(executable, &restart_args())?;
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
        if let Target::Windows { executable } = &target.0 {
            // An in-flight attempt keeps its owner, and so its stage; any other
            // marked stage is a leftover a failed delete could not remove.
            if let Some(install) = executable.parent() {
                zip_swap::sweep_stale_stages(install, record.owner);
            }
        }
        if record.phase == Phase::Installed {
            // Retry a Windows cleanup the acknowledging start could not finish
            // (an antivirus scan holding `<exe>.old`, for example).
            if record
                .apply
                .as_ref()
                .is_some_and(|a| a.target == target.0 && a.swap.is_some())
            {
                let _ = self.cleanup_update(&record);
            }
            return Ok(());
        }
        if !matches!(
            record.phase,
            Phase::ReconciliationRequired | Phase::RestartRequired | Phase::Installing
        ) {
            return Ok(());
        }
        let attempt = record.apply.as_ref().ok_or(Error::Reconciliation)?;
        if attempt.target != target.0 {
            tracing::warn!(target: "launcher.update", event = "reconcile_refused", reason = "other_target", "update record names a different launcher install");
            return Err(Error::Target);
        }
        let expected = record
            .offer
            .as_ref()
            .ok_or(Error::StaleOffer)?
            .version
            .trim_start_matches('v');
        if compiled_version == expected {
            let installed = match &attempt.target {
                Target::Mac { bundle, .. } => bundle,
                Target::Windows { executable } => executable,
            };
            if attempt.replacement.as_deref() != Some(bundle::fingerprint(installed)?.as_str()) {
                tracing::warn!(target: "launcher.update", event = "reconcile_refused", reason = "replacement_mismatch", "installed launcher does not match the applied update");
                return Err(Error::Reconciliation);
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
        // After a handoff only the new compiled version acknowledges. An older
        // process reopening cannot tell a slow restart from a failed one, so it
        // retains reconciliation ownership.
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
        } else if let Target::Windows { executable } = &attempt.target {
            // Called before handoff or for a synchronous spawn failure, never for
            // an uncertain successful handoff. The saved plan says exactly which
            // files moved; without one nothing was renamed yet.
            let install = executable.parent().ok_or(Error::Target)?;
            if let Some(plan) = &attempt.swap {
                zip_swap::roll_back(install, plan)?;
            }
            let _ = backup;
            zip_swap::remove_stage(&stage, record.owner.ok_or(Error::Reconciliation)?)?;
        }
        Ok(())
    }
    fn cleanup_update(&self, record: &Record) -> Result<(), Error> {
        let attempt = record.apply.as_ref().ok_or(Error::Reconciliation)?;
        // An acknowledged record has no owner; a Windows retry uses the plan's.
        let owner = record
            .owner
            .or(attempt.swap.as_ref().map(zip_swap::plan_owner))
            .ok_or(Error::Reconciliation)?;
        let (stage, backup) = attempt.paths(owner, self.state_root())?;
        match &attempt.target {
            Target::Mac { .. } => {
                if stage.exists() {
                    bundle::owned_stage(&stage, owner)?;
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
                    bundle::owned_stage(&stage, owner)?;
                    fs::remove_dir_all(stage).map_err(|_| Error::Replace)?;
                }
            }
            Target::Windows { executable } => {
                // The old process has exited (the restart waited for it), so its
                // `<exe>.old` is no longer a running image and can be deleted.
                let install = executable.parent().ok_or(Error::Target)?;
                match &attempt.swap {
                    Some(plan) => zip_swap::clean_up(install, &stage, plan)?,
                    None => zip_swap::remove_stage(&stage, owner)?,
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
    /// Windows zip-swap seam for engine-to-host lifecycle fault tests. `start`
    /// stands in for the relaunch, so no real launcher process is spawned.
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
