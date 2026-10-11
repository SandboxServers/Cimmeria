//! Whether PhysX System Software is installed, from two independent signs.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The two system lookups detection needs. `native::System` reads the real
/// ones; tests inject fixed answers.
pub trait Probe {
    /// `PhysXLoader.dll` in the 32-bit system directory (`SysWOW64` on 64-bit
    /// Windows, `System32` on 32-bit Windows). The game is 32-bit, so the
    /// 64-bit `System32` copy is not the one that matters.
    fn system_loader(&self) -> bool;
    /// `PhysXCore Path` under `HKLM\SOFTWARE\AGEIA Technologies` in the 32-bit
    /// registry view, when it names a folder that exists. This is the value
    /// the game's loader reads to find the 2.x core.
    fn core_path(&self) -> Option<PathBuf>;
}

/// What detection found. Both are needed: a stale registry value after an
/// uninstall, or a loader DLL some other game dropped in, is not an install.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detection {
    pub loader_present: bool,
    pub core_registered: bool,
}
impl Detection {
    pub fn installed(&self) -> bool {
        self.loader_present && self.core_registered
    }
}

pub fn detect(probe: &dyn Probe) -> Detection {
    Detection {
        loader_present: probe.system_loader(),
        core_registered: probe.core_path().is_some(),
    }
}
