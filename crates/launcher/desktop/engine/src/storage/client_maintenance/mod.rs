//! Reset the game client's cache, or all of its per-user state, under the
//! player's Documents. The client keeps both under `My Games\Firesky`
//! (`cimmeria_launcher_core::client_paths`).
use super::*;
use cimmeria_launcher_core::client_paths::firesky_root;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
#[cfg(test)]
mod tests;

/// A `client_maintenance_command` request from the webview. It names no
/// folder: native code resolves the player's Documents itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMaintenanceCommand {
    /// Report what each reset would remove.
    Inspect { schema_version: u32 },
    /// `Firesky\SGWGame\Cache.en-US` only.
    ResetCache { schema_version: u32 },
    /// The whole `Firesky` tree. Refused unless `confirmed`.
    ResetAllClientState {
        schema_version: u32,
        confirmed: bool,
    },
}
impl ClientMaintenanceCommand {
    fn schema_version(&self) -> u32 {
        match self {
            Self::Inspect { schema_version }
            | Self::ResetCache { schema_version }
            | Self::ResetAllClientState { schema_version, .. } => *schema_version,
        }
    }
}

/// What `Inspect` reports, and what a reset answers with. The paths are for
/// display; the webview never sends one back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientMaintenanceStatus {
    pub schema_version: u32,
    /// `Firesky\SGWGame\Cache.en-US`.
    pub cache: TreeStatus,
    /// The whole `Firesky` tree.
    pub client_state: TreeStatus,
    /// Whether the reset removed anything. `None` for `Inspect`.
    pub removed: Option<bool>,
}

/// One tree the client writes: where it is, and whether it exists now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeStatus {
    pub path: PathBuf,
    pub exists: bool,
}

/// Serializes as one flat code string, like `StorageError`: the storage code
/// itself (`"busy"`, `"unsupported_schema"`, ...), or one of the codes below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientMaintenanceError {
    /// `ResetAllClientState` was sent without `confirmed: true`. Nothing is deleted.
    ConfirmationRequired,
    /// No `USERPROFILE` or `HOME` to resolve the client folder from.
    ProfileUnavailable,
    /// A link sits on the way to the cache, so the reset would delete outside
    /// the Firesky tree. Nothing is deleted.
    UnsafePath,
    /// Admission or state-directory failure, including an unknown schema.
    #[serde(untagged)]
    Storage(StorageError),
}
impl From<StorageError> for ClientMaintenanceError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl DesktopState {
    /// Answer one maintenance request. Both resets take the full mutation
    /// admission (`ensure_no_active_operation`) before anything is deleted.
    pub fn client_maintenance(
        &mut self,
        request: ClientMaintenanceCommand,
    ) -> Result<ClientMaintenanceStatus, ClientMaintenanceError> {
        let Some(firesky) = firesky_root() else {
            tracing::warn!(?request, "client maintenance: no USERPROFILE or HOME");
            return Err(ClientMaintenanceError::ProfileUnavailable);
        };
        let result = self.client_maintenance_in(request, &firesky);
        if let Err(error) = &result {
            tracing::warn!(?request, ?error, firesky = %firesky.display(), "client maintenance refused");
        }
        result
    }

    /// `client_maintenance` against an explicit `Firesky` tree, so tests run
    /// on a temp profile and never touch the player's.
    fn client_maintenance_in(
        &mut self,
        request: ClientMaintenanceCommand,
        firesky: &Path,
    ) -> Result<ClientMaintenanceStatus, ClientMaintenanceError> {
        if request.schema_version() != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        let cache = cache_dir(firesky);
        match request {
            ClientMaintenanceCommand::Inspect { .. } => Ok(status(firesky, None)),
            ClientMaintenanceCommand::ResetCache { .. } => {
                self.ensure_no_active_operation()?;
                ensure_no_linked_ancestor(firesky)?;
                let removed = remove_tree(&cache).map_err(|error| removal_failed(&cache, error))?;
                Ok(status(firesky, Some(removed)))
            }
            ClientMaintenanceCommand::ResetAllClientState { confirmed, .. } => {
                // The confirmation is checked before admission so an
                // unconfirmed request never reaches a deletion.
                if !confirmed {
                    return Err(ClientMaintenanceError::ConfirmationRequired);
                }
                self.ensure_no_active_operation()?;
                // A `Firesky` that is itself a link is removed as the link;
                // `remove_tree` never follows it.
                let removed =
                    remove_tree(firesky).map_err(|error| removal_failed(firesky, error))?;
                Ok(status(firesky, Some(removed)))
            }
        }
    }
}

/// `Firesky\SGWGame\Cache.en-US`: the server-pushed PAK overrides the client
/// reads before the bundled PAKs.
fn cache_dir(firesky: &Path) -> PathBuf {
    firesky.join("SGWGame").join("Cache.en-US")
}

fn status(firesky: &Path, removed: Option<bool>) -> ClientMaintenanceStatus {
    ClientMaintenanceStatus {
        schema_version: 1,
        cache: tree_status(&cache_dir(firesky)),
        client_state: tree_status(firesky),
        removed,
    }
}

fn tree_status(path: &Path) -> TreeStatus {
    TreeStatus {
        path: path.to_path_buf(),
        exists: fs::symlink_metadata(path).is_ok(),
    }
}

/// Refuse a cache reset when `Firesky` or `SGWGame` is a link: the cache
/// would then be deleted through the link, outside the Firesky tree.
fn ensure_no_linked_ancestor(firesky: &Path) -> Result<(), ClientMaintenanceError> {
    for ancestor in [firesky.to_path_buf(), firesky.join("SGWGame")] {
        if fs::symlink_metadata(&ancestor).is_ok_and(|meta| is_link(&meta)) {
            return Err(ClientMaintenanceError::UnsafePath);
        }
    }
    Ok(())
}

/// A removal that stopped part-way (a file the running game holds open, say)
/// leaves the rest in place; the next reset finishes it. The flat code is
/// `io`; the log keeps the path and the OS error.
fn removal_failed(path: &Path, error: io::Error) -> ClientMaintenanceError {
    tracing::warn!(path = %path.display(), %error, "client maintenance: removal stopped part-way");
    StorageError::Io.into()
}

/// Remove `path` and everything under it. Returns `false` when nothing was
/// there.
fn remove_tree(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            remove_entry(path, &meta)?;
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Recurse without following links: a link is removed as itself, so its
/// target is never touched.
fn remove_entry(path: &Path, meta: &fs::Metadata) -> io::Result<()> {
    if is_link(meta) {
        // A directory link needs `remove_dir` on Windows, a file link `remove_file`.
        return fs::remove_dir(path).or_else(|_| fs::remove_file(path));
    }
    if meta.is_dir() {
        for entry in fs::read_dir(path)? {
            let child = entry?.path();
            remove_entry(&child, &fs::symlink_metadata(&child)?)?;
        }
        return fs::remove_dir(path);
    }
    match fs::remove_file(path) {
        // Windows refuses to delete a read-only file. This is a regular file
        // (links returned above), so the permission change touches only it.
        Err(error)
            if error.kind() == io::ErrorKind::PermissionDenied && meta.permissions().readonly() =>
        {
            let mut writable = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            writable.set_readonly(false);
            fs::set_permissions(path, writable)?;
            fs::remove_file(path)
        }
        other => other,
    }
}

/// A symlink, or a Windows junction. std does not always report a junction
/// as a symlink, so the reparse-point attribute is checked too.
fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}
