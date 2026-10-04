//! Separate prefix ownership for installation and repair extraction work.
use super::*;
use std::io::Write;
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairOwner {
    schema_version: u32,
    work: ExtractionWork,
}
pub(super) fn claim_work_prefix(
    state_root: &Path,
    work: &ExtractionWork,
) -> Result<(PathBuf, File), WineError> {
    if work.operation_id == work.installation.operation_id {
        claim_prefix(&state_root.join("wine-prefixes"), &work.installation)
    } else {
        claim_record(
            &state_root.join("wine-repair-prefixes"),
            work.operation_id,
            &RepairOwner {
                schema_version: 1,
                work: work.clone(),
            },
        )
    }
}
pub(super) fn claim_prefix(
    root: &Path,
    intent: &InstallIntent,
) -> Result<(PathBuf, File), WineError> {
    claim_record(root, intent.operation_id, intent)
}
fn claim_record(
    root: &Path,
    id: Uuid,
    record: &impl serde::Serialize,
) -> Result<(PathBuf, File), WineError> {
    let io = |_| WineError::Invalid;
    std::fs::create_dir_all(root).map_err(io)?;
    if root.canonicalize().map_err(io)? != root {
        return Err(WineError::Invalid);
    }
    let owned = root.join(id.to_string());
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
        .write_all(&serde_json::to_vec(record).map_err(|_| WineError::Invalid)?)
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
