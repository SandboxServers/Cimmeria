//! Quiescence gate for explicit repair recovery on a native blocking thread.
use super::*;
use crate::{HelperPhase, HelperResult, IntentError, StorageError};
use recovery::{host_absent, StoppedPrefix};

/// Caller retains DesktopState and installation ownership through filesystem
/// reconciliation. Completion is required for promotion/cleanup, not abandonment.
pub(crate) fn stop(
    state: &DesktopState,
    id: Uuid,
    require_completed: bool,
) -> Result<StoppedPrefix, IntentError> {
    let plan = state.repair_plan()?.ok_or(StorageError::Corrupt)?;
    if plan.id != id {
        return Err(StorageError::Corrupt.into());
    }
    let work = state.extraction_work(id)?;
    let ExtractionBackend::Wine { runtime_sha256, .. } = work.installation.backend else {
        return Err(StorageError::Corrupt.into());
    };
    if hex(&runtime_sha256) != mac_runtime::ARCHIVE_SHA256 {
        return Err(StorageError::Corrupt.into());
    }
    let record = state.helper_record(id)?;
    if require_completed
        && !record.as_ref().is_some_and(|r| {
            r.phase
                == (HelperPhase::Finished {
                    result: HelperResult::Completed,
                })
        })
    {
        return Err(StorageError::Corrupt.into());
    }
    if let Some(record) = &record {
        host_absent(record)?;
    }
    let root = state
        .state_root()
        .join("wine-repair-prefixes")
        .join(id.to_string());
    match std::fs::symlink_metadata(&root) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && record.is_none() => {
            return Ok(StoppedPrefix {
                _prefix_owner: None,
                _runtime_owner: None,
            });
        }
        Err(_) => return Err(StorageError::Corrupt.into()),
        Ok(_) => (),
    }
    let owner = prefix::open_repair(&root, &work)?;
    // No launch journal means no extraction process could have been dispatched.
    if record.is_none() {
        return Ok(StoppedPrefix {
            _prefix_owner: Some(owner),
            _runtime_owner: None,
        });
    }
    let bottle = root.join("bottle");
    if bottle
        .canonicalize()
        .map_err(|_| StorageError::UnsafeFile)?
        != bottle
        || !std::fs::symlink_metadata(&bottle)
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
    {
        return Err(StorageError::UnsafeFile.into());
    }
    let (runtime, runtime_owner) =
        mac_runtime::verified_cached(&state.state_root().join("runtimes"))
            .map_err(|_| StorageError::Io)?;
    let env = environment(&runtime, &bottle).map_err(|_| StorageError::Corrupt)?;
    tokio::runtime::Handle::try_current()
        .map_err(|_| StorageError::Io)?
        .block_on(stop_prefix(&runtime, &env))
        .map_err(|_| StorageError::Io)?;
    Ok(StoppedPrefix {
        _prefix_owner: Some(owner),
        _runtime_owner: Some(runtime_owner),
    })
}
#[cfg(test)]
mod tests;
