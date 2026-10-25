//! The launcher's private Proton prefix and its setup stamp (C-2, D-PP5).
//!
//! One prefix lives under each state root. Its lock is held for as long as the
//! `Prefix` value lives, so two launchers never run Proton in the same prefix.
//! The stamp records which Proton build last set the prefix up; a different
//! build, or no stamp, means the setup has to run again.

use std::fs::{File, TryLockError};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::discovery::ProtonInstall;

/// The file in `compat` that holds the [`Stamp`].
pub const STAMP_FILE: &str = "cimmeria-setup.json";

/// A claimed prefix. The lock file stays open until this value is dropped.
#[derive(Debug)]
pub struct Prefix {
    /// `STEAM_COMPAT_DATA_PATH`: the `compatdata` folder that holds the prefix.
    pub compat: PathBuf,
    /// The Wine prefix itself, `compat/pfx`.
    pub pfx: PathBuf,
    _lock: File,
}

/// What the last good setup of the prefix left behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stamp {
    /// Always 1 for now.
    pub schema_version: u32,
    /// `ProtonInstall::version` used by the last good setup.
    pub proton_version: String,
    #[serde(default)]
    pub physx: PhysxState,
}

/// The state of the PhysX install inside the prefix.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysxState {
    #[default]
    NotInstalled,
    Installed,
    Failed,
}

/// Why a prefix could not be used.
#[derive(Debug, thiserror::Error)]
pub enum ProtonError {
    #[error("no usable Proton")]
    Missing,
    #[error("the Proton prefix is in use")]
    Busy,
    #[error("Proton setup failed: {0}")]
    Setup(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Claims the private prefix under `state_root`, creating its folders and
/// taking the exclusive lock. `Busy` when another claim holds the lock.
pub fn claim(state_root: &Path) -> Result<Prefix, ProtonError> {
    let proton = state_root.join("proton");
    std::fs::create_dir_all(&proton)?;
    let compat = proton.join("compatdata");
    private_dir(&compat)?;
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(proton.join("prefix.lock"))?;
    lock.try_lock().map_err(|err| match err {
        TryLockError::WouldBlock => ProtonError::Busy,
        TryLockError::Error(io) => ProtonError::Io(io),
    })?;
    let pfx = compat.join("pfx");
    Ok(Prefix {
        compat,
        pfx,
        _lock: lock,
    })
}

/// Creates `path` with mode 0700 when it is missing. An existing folder is kept.
fn private_dir(path: &Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    match builder.create(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(err),
    }
}

impl Prefix {
    /// The stamp, or None when it is missing or unreadable.
    pub fn stamp(&self) -> Option<Stamp> {
        let text = std::fs::read_to_string(self.compat.join(STAMP_FILE)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Writes the stamp through a temporary file, then renames it into place.
    pub fn write_stamp(&self, stamp: &Stamp) -> Result<(), ProtonError> {
        let text = serde_json::to_string_pretty(stamp)
            .map_err(|_| ProtonError::Setup("the stamp could not be encoded"))?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.compat)?;
        temp.write_all(text.as_bytes())?;
        temp.as_file().sync_all()?;
        temp.persist(self.compat.join(STAMP_FILE))
            .map_err(|err| ProtonError::Io(err.error))?;
        Ok(())
    }

    /// True when there is no stamp, or the stamp names another Proton version.
    pub fn needs_setup(&self, install: &ProtonInstall) -> bool {
        self.stamp()
            .is_none_or(|stamp| stamp.proton_version != install.version)
    }

    /// The Windows profile folder inside the prefix.
    pub fn profile_root(&self) -> PathBuf {
        self.pfx.join("drive_c").join("users").join("steamuser")
    }
}

#[cfg(test)]
#[path = "prefix_tests.rs"]
mod tests;
