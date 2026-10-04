//! Operation-specific ownership evidence that moves with each renamed tree.
use super::*;
use std::io::Write;
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Role {
    Original,
    Replacement,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema_version: u32,
    plan: Plan,
    role: Role,
}
fn name(plan: &Plan) -> String {
    format!(".cimmeria-repair-tree-{}.json", plan.id)
}
pub(super) fn absent(tree: &Path, plan: &Plan) -> Result<(), StorageError> {
    match std::fs::symlink_metadata(tree.join(name(plan))) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StorageError::Io),
        Ok(_) => Err(StorageError::UnsafeFile),
    }
}
pub(super) fn write(tree: &Path, plan: &Plan, role: Role) -> Result<(), StorageError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tree.join(name(plan)))
        .map_err(|_| StorageError::Io)?;
    file.write_all(
        &serde_json::to_vec(&Marker {
            schema_version: 1,
            plan: plan.clone(),
            role,
        })
        .map_err(|_| StorageError::Corrupt)?,
    )
    .map_err(|_| StorageError::Io)?;
    file.sync_all()
        .map_err(|_| StorageError::PersistenceUncertain)?;
    commit::sync(tree)
}
pub(super) fn read_role(tree: &Path, plan: &Plan) -> Result<Option<Role>, StorageError> {
    if !directory_or_absent(tree)? {
        return Ok(None);
    }
    let path = tree.join(name(plan));
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| StorageError::Corrupt)?;
    ordinary(&metadata)?;
    if !metadata.is_file() {
        return Err(StorageError::UnsafeFile);
    }
    let marker: Marker = read(&path)?.ok_or(StorageError::Corrupt)?;
    if marker.schema_version != 1 || marker.plan != *plan {
        return Err(StorageError::Corrupt);
    }
    failed_cleanup::validate_tree(tree)?;
    Ok(Some(marker.role))
}
