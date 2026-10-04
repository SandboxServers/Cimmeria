//! Stop only the extraction prefix named by a durable install intent.
//! Recorded PIDs are checked for absence, never used as kill targets.
use super::*;
use crate::{HelperPhase, HelperResult, IntentError, StorageError};

pub(crate) struct StoppedPrefix {
    pub(super) _prefix_owner: Option<File>,
    pub(super) _runtime_owner: Option<File>,
}

/// Called on the native blocking command thread with DesktopState locked.
/// The guards survive through content inspection and the reconciliation commit.
pub(crate) fn stop_for_recovery(state: &DesktopState) -> Result<StoppedPrefix, IntentError> {
    let intent = state.install_intent()?.ok_or(StorageError::Corrupt)?;
    stop_for_install(state, &intent)
}

pub(crate) fn stop_for_install(
    state: &DesktopState,
    intent: &InstallIntent,
) -> Result<StoppedPrefix, IntentError> {
    let ExtractionBackend::Wine { runtime_sha256, .. } = intent.backend else {
        return Err(StorageError::Corrupt.into());
    };
    if hex(&runtime_sha256) != mac_runtime::ARCHIVE_SHA256 {
        return Err(StorageError::Corrupt.into());
    }
    let record = state.helper_record_for_install(intent)?;
    // No durable launch intent means the supervisor could not have spawned.
    if record.is_none() {
        return Ok(StoppedPrefix {
            _prefix_owner: None,
            _runtime_owner: None,
        });
    }
    let record = record.unwrap();
    host_absent(&record)?;
    let root = state
        .state_root()
        .join("wine-prefixes")
        .join(intent.operation_id.to_string());
    let owner = open_owner(&root, intent)?;
    let prefix = root.join("bottle");
    if prefix
        .canonicalize()
        .map_err(|_| StorageError::UnsafeFile)?
        != prefix
        || !std::fs::symlink_metadata(&prefix).is_ok_and(|m| m.is_dir())
    {
        return Err(StorageError::UnsafeFile.into());
    }
    let (runtime, runtime_owner) =
        mac_runtime::verified_cached(&state.state_root().join("runtimes"))
            .map_err(|_| StorageError::Io)?;
    let environment = environment(&runtime, &prefix).map_err(|_| StorageError::Corrupt)?;
    let handle = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    handle
        .block_on(stop_prefix(&runtime, &environment))
        .map_err(|_| StorageError::Io)?;
    Ok(StoppedPrefix {
        _prefix_owner: Some(owner),
        _runtime_owner: Some(runtime_owner),
    })
}

pub(super) fn host_absent(record: &crate::HelperRecord) -> Result<(), StorageError> {
    // Unobserved startup/extraction descendants need dedicated crash validation.
    // Only an observed terminal helper result can enter the current stop path.
    if !matches!(
        record.phase,
        HelperPhase::Finished {
            result: HelperResult::Completed
                | HelperResult::Cancelled
                | HelperResult::Failed
                | HelperResult::NotStarted
        }
    ) {
        return Err(StorageError::InUse);
    }
    match record.host_pid {
        Some(pid) if pid <= i32::MAX as u32 => {
            // Signal zero checks liveness only. Reused/live PIDs conservatively
            // keep recovery gated; EPERM is not evidence of process death.
            let result = unsafe { libc::kill(pid as i32, 0) };
            if result != -1 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(StorageError::InUse);
            }
            Ok(())
        }
        None if record.phase
            == (HelperPhase::Finished {
                result: HelperResult::NotStarted,
            }) =>
        {
            Ok(())
        }
        // Crash between spawn and PID publication: never guess or use -w alone,
        // because an unrecorded host may still start its Wine server afterward.
        _ => Err(StorageError::Corrupt),
    }
}

fn open_owner(root: &Path, intent: &InstallIntent) -> Result<File, StorageError> {
    if root.canonicalize().map_err(|_| StorageError::UnsafeFile)? != root {
        return Err(StorageError::UnsafeFile);
    }
    let marker = root.join("owner.json");
    if !std::fs::symlink_metadata(&marker).is_ok_and(|m| m.is_file() && m.len() <= 65536) {
        return Err(StorageError::UnsafeFile);
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(marker)
        .map_err(|_| StorageError::Io)?;
    file.try_lock().map_err(|_| StorageError::InUse)?;
    let mut bytes = Vec::new();
    (&mut file)
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| StorageError::Io)?;
    let owner: InstallIntent = serde_json::from_slice(&bytes).map_err(|_| StorageError::Corrupt)?;
    if bytes.len() > 65536 || owner != *intent {
        return Err(StorageError::Corrupt);
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_or_unrecorded_hosts_are_never_stopped_by_pid() {
        let mut record = crate::HelperRecord {
            schema_version: 1,
            operation_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            intent_digest: [0; 32],
            phase: HelperPhase::Finished {
                result: HelperResult::Completed,
            },
            host_pid: Some(std::process::id()),
        };
        assert_eq!(host_absent(&record), Err(StorageError::InUse));
        record.host_pid = None;
        record.phase = HelperPhase::LaunchIntent;
        assert_eq!(host_absent(&record), Err(StorageError::InUse));
        record.phase = HelperPhase::Finished {
            result: HelperResult::NotStarted,
        };
        assert_eq!(host_absent(&record), Ok(()));
        record.host_pid = Some(u32::MAX);
        assert_eq!(host_absent(&record), Err(StorageError::Corrupt));
    }
    #[test]
    fn owner_lock_and_identity_are_required() {
        let (_root, state, _, _) = super::super::tests::fixture();
        let state = state.lock().unwrap();
        let intent = state.install_intent().unwrap().unwrap();
        let prefix_root = state.state_root().join("wine-prefixes");
        let (prefix, guard) = claim_prefix(&prefix_root, &intent).unwrap();
        let root = prefix.parent().unwrap();
        assert!(matches!(
            open_owner(root, &intent),
            Err(StorageError::InUse)
        ));
        drop(guard);
        let mut different = intent.clone();
        different.operation_id = Uuid::new_v4();
        assert!(matches!(
            open_owner(root, &different),
            Err(StorageError::Corrupt)
        ));
        assert!(open_owner(root, &intent).is_ok());
    }
    #[tokio::test]
    async fn stop_wait_failures_and_timeouts_cannot_produce_a_stopped_guard() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("bin")).unwrap();
        let server = root.path().join("bin/wineserver");
        std::fs::write(&server, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&server, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(stop_prefix_with_limit(
            root.path(),
            &BTreeMap::new(),
            std::time::Duration::from_secs(1)
        )
        .await
        .is_err());
        std::fs::write(&server, "#!/bin/sh\nexec /bin/sleep 60\n").unwrap();
        assert!(stop_prefix_with_limit(
            root.path(),
            &BTreeMap::new(),
            std::time::Duration::from_millis(20)
        )
        .await
        .is_err());
    }
}
