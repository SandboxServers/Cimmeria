//! Whether the PhysX core the game loads is installed.
//!
//! The game ships its own `PhysXLoader.dll` next to `SGW.exe`, and DLL search
//! finds that copy first, so a loader in the system directory is irrelevant.
//! The bundled loader reads REG_SZ `PhysXCore Path` and loads
//! `<PhysXCore Path>\v2.6.3\PhysXCore.dll` for SDK `0x02060300`
//! (`docs/physx-probe.md`, core lookup at `0x10001070`). That file is the
//! evidence: a registered folder without the 2.6.3 core, such as one left by a
//! newer PhysX release, still fails `NxCreatePhysicsSDK`.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The 2.x core the game's loader asks for, relative to `PhysXCore Path`.
pub const CORE_RELATIVE: &str = r"v2.6.3\PhysXCore.dll";

/// The registry lookup detection needs. `native::System` reads the real one;
/// tests inject a fixed answer.
pub trait Probe {
    /// `PhysXCore Path` under `HKLM\SOFTWARE\AGEIA Technologies` in the 32-bit
    /// registry view, when it is set and non-empty. This is the value the
    /// game's loader reads to find the 2.x core.
    fn core_path(&self) -> Option<PathBuf>;
}

/// What detection found. Only `core_present` makes an install: a registered
/// path without the 2.6.3 core is not one the game can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detection {
    /// The registry value is set.
    pub core_registered: bool,
    /// `<PhysXCore Path>\v2.6.3\PhysXCore.dll` is a file.
    pub core_present: bool,
}
impl Detection {
    pub fn installed(&self) -> bool {
        self.core_present
    }
}

pub fn detect(probe: &dyn Probe) -> Detection {
    let core = probe.core_path();
    Detection {
        core_registered: core.is_some(),
        core_present: core.is_some_and(|root| is_file(&core_file(&root))),
    }
}

/// The file the loader would `LoadLibraryA`. `Path::join` handles a value with
/// or without its trailing separator.
pub fn core_file(root: &Path) -> PathBuf {
    CORE_RELATIVE
        .split('\\')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

fn is_file(path: &Path) -> bool {
    // A metadata check, not LoadLibrary: a 32-bit DLL cannot load into the
    // 64-bit launcher, and loading would run its entry point.
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file())
}
