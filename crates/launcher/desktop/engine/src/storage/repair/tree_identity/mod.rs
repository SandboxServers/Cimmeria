//! Operation-specific ownership evidence that moves with each renamed tree.
use super::*;
use std::io::Write;
pub(crate) trait TreePlan:
    Serialize + serde::de::DeserializeOwned + PartialEq + Clone
{
    fn marker_name(&self) -> String;
}
impl TreePlan for Plan {
    fn marker_name(&self) -> String {
        format!(".cimmeria-repair-tree-{}.json", self.id)
    }
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role {
    Original,
    Replacement,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Marker<P> {
    schema_version: u32,
    plan: P,
    role: Role,
}
pub(crate) fn name<P: TreePlan>(plan: &P) -> String {
    plan.marker_name()
}
pub(crate) fn absent<P: TreePlan>(tree: &Path, plan: &P) -> Result<(), StorageError> {
    match std::fs::symlink_metadata(tree.join(name(plan))) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StorageError::Io),
        Ok(_) => Err(StorageError::UnsafeFile),
    }
}
pub(crate) fn write<P: TreePlan>(tree: &Path, plan: &P, role: Role) -> Result<(), StorageError> {
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
pub(crate) fn read_role<P: TreePlan>(tree: &Path, plan: &P) -> Result<Option<Role>, StorageError> {
    if !directory_or_absent(tree)? {
        return Ok(None);
    }
    let path = tree.join(name(plan));
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| StorageError::Corrupt)?;
    ordinary(&metadata)?;
    if !metadata.is_file() {
        return Err(StorageError::UnsafeFile);
    }
    let marker: Marker<P> = read(&path)?.ok_or(StorageError::Corrupt)?;
    if marker.schema_version != 1 || marker.plan != *plan {
        return Err(StorageError::Corrupt);
    }
    failed_cleanup::validate_tree(tree)?;
    Ok(Some(marker.role))
}
