//! PhysX System Software on native Windows (D-LX13). The 32-bit game's own
//! `PhysXLoader.dll` loads `v2.6.3\PhysXCore.dll` from the path the system
//! software registers, so a machine without it fails `NxCreatePhysicsSDK`
//! (#1121). The launcher detects that core file and, when it is missing, runs the
//! same vendor MSI the macOS path verifies, elevated through UAC. It does not
//! write the `enableLocalPhysXCore` registry value (tied to a MAC address).
//!
//! Detection and the installer are injected, so the decisions here are tested
//! without a registry, UAC or msiexec. `native` holds the Windows FFI.
use crate::prerequisites::{physx_msi, PHYSX_EXE_BYTES};
use crate::{DesktopState, StorageError};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

mod detect;
#[cfg(windows)]
pub mod native;
pub use detect::{core_file, detect, Detection, Probe, CORE_RELATIVE};

/// The retained vendor installer, relative to the installation's `game` folder.
/// Install extraction keeps `Data/Prerequisites` here as inert data.
pub const PACKAGE: &str = ".cimmeria-prerequisites/PhysX/PhysX_7.11.13_SystemSoftware.exe";
/// Scratch for the extracted MSI, under the launcher state root.
const SCRATCH: &str = "physx-setup";
/// msiexec success codes: done, and done but Windows wants a restart.
const ERROR_SUCCESS: u32 = 0;
const ERROR_SUCCESS_REBOOT_REQUIRED: u32 = 3010;
/// The player pressed Cancel inside the installer's own wizard.
const ERROR_INSTALL_USEREXIT: u32 = 1602;
/// `ERROR_PRODUCT_VERSION`: another version of this product is already
/// installed, so Windows Installer refuses to run this one.
const ERROR_PRODUCT_VERSION: u32 = 1638;

/// A `physx_command` request from the webview. It names no path, URL or hash:
/// native code finds the package and pins its identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum PhysxCommand {
    Inspect { schema_version: u32 },
    Install { schema_version: u32 },
}
impl PhysxCommand {
    pub fn schema_version(&self) -> u32 {
        match self {
            Self::Inspect { schema_version } | Self::Install { schema_version } => *schema_version,
        }
    }
}

/// How the last Install press ended. Path-free and safe for IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstallResult {
    /// The installer succeeded and detection now finds the system software.
    Installed { restart_required: bool },
    /// The player declined the UAC prompt. Nothing ran.
    CancelledAtUac {},
    /// The player cancelled the vendor installer's own wizard.
    CancelledInInstaller {},
    /// msiexec exited with an error code.
    Failed { exit_code: u32 },
    /// msiexec reported success, but detection still finds nothing.
    NotDetected { exit_code: u32 },
    /// Exit code 1638: another PhysX version is installed and blocks this
    /// installer, and the 2.6.3 core the game loads is still missing.
    OtherVersionBlocks {},
    /// Windows could not start the elevated installer at all.
    StartFailed { win32_error: u32 },
}

/// What `Inspect` and `Install` answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhysxStatus {
    pub schema_version: u32,
    /// False off native Windows: the macOS path sets PhysX up inside Wine.
    pub supported: bool,
    pub detection: Option<Detection>,
    /// The retained vendor installer exists at its expected size. Its hash is
    /// checked only when Install reads it.
    pub installer_available: bool,
    pub installing: bool,
    pub last_result: Option<InstallResult>,
}

/// Serializes as one flat code string, like `StorageError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysxError {
    /// Not native Windows.
    Unavailable,
    /// No installed game, or it has no retained PhysX installer.
    InstallerUnavailable,
    /// The retained installer, or the MSI inside it, is not the pinned one.
    PackageMismatch,
    #[serde(untagged)]
    Storage(StorageError),
}
impl From<StorageError> for PhysxError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

/// Starts the vendor MSI elevated and waits for it.
pub trait Elevate {
    /// The installer's exit code, or why it never ran.
    fn run_msi(&self, msi: &Path) -> Result<u32, ElevateError>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevateError {
    /// `ERROR_CANCELLED` from `ShellExecuteExW`: the UAC prompt was declined.
    Cancelled,
    Win32(u32),
}

/// Where Install reads the package and writes the extracted MSI. Native-chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallSource {
    pub package: PathBuf,
    pub scratch: PathBuf,
}

impl DesktopState {
    /// The retained installer for the current installation, if it is there.
    /// Read-only: Inspect calls it without the mutation admission.
    pub fn physx_package(&mut self) -> Result<Option<PathBuf>, StorageError> {
        let Some(installed) = self.installed_content()? else {
            return Ok(None);
        };
        let package = installed.intent.destination.join("game").join(PACKAGE);
        let present = std::fs::symlink_metadata(&package)
            .is_ok_and(|meta| meta.is_file() && meta.len() == PHYSX_EXE_BYTES as u64);
        Ok(present.then_some(package))
    }

    /// Admission for Install: the same gate every other mutation takes, so it
    /// refuses while the updater, an install, Play, Repair or any operation is
    /// nonterminal. Writes nothing itself.
    pub fn physx_install_source(&mut self) -> Result<InstallSource, PhysxError> {
        self.ensure_no_active_operation()?;
        let package = self.physx_package()?.ok_or_else(|| {
            tracing::warn!(
                "PhysX install refused: no retained vendor installer in the installation"
            );
            PhysxError::InstallerUnavailable
        })?;
        Ok(InstallSource {
            package,
            scratch: self.state_root().join(SCRATCH),
        })
    }
}

/// Verify the package, extract its MSI, run it elevated, then detect again.
/// A package or MSI that is not the pinned one is refused before anything runs.
pub fn install(
    source: &InstallSource,
    elevate: &dyn Elevate,
    probe: &dyn Probe,
) -> Result<InstallResult, PhysxError> {
    let bytes = read_package(&source.package)?;
    let payload = physx_msi(&bytes).map_err(|error| {
        tracing::warn!(
            ?error,
            "PhysX install refused: vendor installer does not match the pinned package"
        );
        PhysxError::PackageMismatch
    })?;
    std::fs::create_dir_all(&source.scratch).map_err(|error| {
        tracing::warn!(%error, "PhysX install: could not create the scratch folder");
        StorageError::Io
    })?;
    let msi = source
        .scratch
        .join(format!("physx-7.11.13-{}.msi", uuid::Uuid::new_v4()));
    let result = write_msi(&msi, payload).map(|_guard| {
        // `_guard` keeps a read-only, deny-write/delete handle open while msiexec runs.
        let code = elevate.run_msi(&msi);
        outcome(code, detect(probe))
    });
    if let Err(error) = std::fs::remove_file(&msi) {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(%error, "PhysX install: could not remove the extracted MSI");
        }
    }
    match &result {
        Ok(InstallResult::Installed { restart_required }) => {
            tracing::info!(restart_required, "PhysX System Software installed")
        }
        Ok(other) => tracing::warn!(result = ?other, "PhysX install did not complete"),
        Err(error) => tracing::warn!(?error, "PhysX install failed before the installer ran"),
    }
    result
}

/// Map the installer's end, and what detection finds afterwards, to a result.
pub fn outcome(code: Result<u32, ElevateError>, after: Detection) -> InstallResult {
    match code {
        Err(ElevateError::Cancelled) => InstallResult::CancelledAtUac {},
        Err(ElevateError::Win32(win32_error)) => InstallResult::StartFailed { win32_error },
        Ok(ERROR_INSTALL_USEREXIT) => InstallResult::CancelledInInstaller {},
        Ok(exit_code @ (ERROR_SUCCESS | ERROR_SUCCESS_REBOOT_REQUIRED)) => {
            if after.installed() {
                InstallResult::Installed {
                    restart_required: exit_code == ERROR_SUCCESS_REBOOT_REQUIRED,
                }
            } else {
                InstallResult::NotDetected { exit_code }
            }
        }
        // The installed release may already carry the 2.6.3 core; only
        // detection can say whether the refusal matters.
        Ok(ERROR_PRODUCT_VERSION) if after.installed() => InstallResult::Installed {
            restart_required: false,
        },
        Ok(ERROR_PRODUCT_VERSION) => InstallResult::OtherVersionBlocks {},
        Ok(exit_code) => InstallResult::Failed { exit_code },
    }
}

fn read_package(path: &Path) -> Result<Vec<u8>, PhysxError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Deny writes while the package is read and hashed.
        options.share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ);
    }
    let mut file = options.open(path).map_err(|error| {
        tracing::warn!(%error, "PhysX install: could not open the retained vendor installer");
        PhysxError::InstallerUnavailable
    })?;
    let meta = file.metadata().map_err(|_| StorageError::Io)?;
    if !meta.is_file() || meta.len() != PHYSX_EXE_BYTES as u64 {
        tracing::warn!(
            len = meta.len(),
            "PhysX install refused: vendor installer has the wrong size"
        );
        return Err(PhysxError::PackageMismatch);
    }
    let mut bytes = Vec::with_capacity(PHYSX_EXE_BYTES);
    (&mut file)
        .take(PHYSX_EXE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            tracing::warn!(%error, "PhysX install: could not read the vendor installer");
            StorageError::Io
        })?;
    Ok(bytes)
}

/// Write the MSI, then reopen it read-only and recheck its bytes. The Windows
/// Installer may refuse to share with a writable handle, so the returned guard
/// is read-only and denies writers and deletion until it is dropped.
fn write_msi(path: &Path, payload: &[u8]) -> Result<std::fs::File, PhysxError> {
    let io = |error: std::io::Error| {
        tracing::warn!(%error, "PhysX install: could not write the extracted MSI");
        PhysxError::Storage(StorageError::Io)
    };
    let mut options = std::fs::OpenOptions::new();
    options.create_new(true).read(true).write(true);
    share_read_only(&mut options);
    let mut output = options.open(path).map_err(io)?;
    output.write_all(payload).map_err(io)?;
    output.sync_all().map_err(io)?;
    drop(output);
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    share_read_only(&mut options);
    let mut guard = options.open(path).map_err(io)?;
    let mut written = Vec::with_capacity(payload.len());
    (&mut guard)
        .take(payload.len() as u64 + 1)
        .read_to_end(&mut written)
        .map_err(io)?;
    if written != payload {
        tracing::warn!("PhysX install refused: the extracted MSI changed on disk");
        return Err(PhysxError::PackageMismatch);
    }
    Ok(guard)
}

fn share_read_only(options: &mut std::fs::OpenOptions) {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ);
    }
    #[cfg(not(windows))]
    let _ = options;
}

#[cfg(test)]
mod tests;
