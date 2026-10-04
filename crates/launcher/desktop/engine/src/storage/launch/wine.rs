//! Interactive game environment differs intentionally from headless extraction.
use super::*;
use crate::{helper_supervisor::HelperCommand, mac_wine};
use cimmeria_runtime_probe::game_launch::Request;
/// Guards that live through the helper's full guest lifetime.
pub(super) struct Ownership {
    // Present only when the staged loader runs the game; see `mac_wine::desktop_host`.
    // Declared first: its keeper is released before the runtime and prefix locks.
    _desktop: Option<mac_wine::desktop_host::DesktopHost>,
    _resources: mac_wine::prerequisites::prefix::Resources,
}

pub(super) fn prepare(
    plan: &Plan,
    state_root: &Path,
    cancelled: &dyn Fn() -> bool,
) -> Result<(HelperCommand, Request, Ownership), IntentError> {
    let runtime = plan.runtime.as_ref().ok_or(StorageError::Corrupt)?;
    let resources = mac_wine::prerequisites::prefix::Resources::reopen(runtime, state_root)?;
    let binaries = preparation::prepare(plan)?;
    let graphics = plan
        .resources
        .graphics
        .as_ref()
        .ok_or(StorageError::Corrupt)?;
    graphics.d3d9.stage(&binaries.join("d3d9.dll"))?;
    let environment = game_environment(&resources.runtime, &resources.prefix, graphics)?;
    let guest = |p: &Path| {
        mac_wine::paths::guest(p)
            .map(PathBuf::from)
            .map_err(|_| StorageError::UnsafeFile)
    };
    let request = Request {
        schema_version: 1,
        operation_id: plan.id,
        exe: guest(&binaries.join("SGW.exe"))?,
        directory: guest(&binaries)?,
        dlls: plan
            .resources
            .client_patches
            .iter()
            .map(|p| guest(p.path()))
            .collect::<Result<_, _>>()?,
    };
    let helper = guest(plan.resources.helper.path())?.into_os_string();
    // Last: a desktop host started here is retained, and released by any return.
    let (executable, desktop) =
        mac_wine::app_identity::launch(&resources.runtime, state_root, &environment, cancelled);
    let spec = HelperCommand {
        executable,
        arguments: vec![helper],
        directory: binaries,
        environment,
    };
    let ownership = Ownership {
        _desktop: desktop,
        _resources: resources,
    };
    Ok((spec, request, ownership))
}

// DXVK's limiter; the client has no vsync setting and nothing else bounds its render loop.
const FRAME_RATE_LIMIT: &str = "30";

// Called only after Resources::reopen verifies and locks the pinned runtime.
fn game_environment(
    runtime: &Path,
    prefix: &Path,
    graphics: &Graphics,
) -> Result<std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>, IntentError> {
    let descriptor = runtime.join("lib/vulkan/icd.d/MoltenVK_icd.json");
    if !std::fs::symlink_metadata(&descriptor).is_ok_and(|metadata| metadata.is_file())
        || descriptor
            .canonicalize()
            .map_err(|_| StorageError::UnsafeFile)?
            != descriptor
    {
        return Err(StorageError::UnsafeFile.into());
    }
    let mut environment =
        mac_wine::environment(runtime, prefix).map_err(|_| StorageError::Corrupt)?;
    environment.insert(
        "WINEDLLOVERRIDES".into(),
        "d3d9=n;winemenubuilder.exe,mscoree,mshtml=d".into(),
    );
    environment.insert("CX_FWD_COMPAT_GL_CTX".into(), "1".into());
    environment.insert("DXVK_FRAME_RATE".into(), FRAME_RATE_LIMIT.into());
    if let Some((executable, _)) = &graphics.rosetta_x87 {
        environment.insert(
            "ROSETTA_X87_PATH".into(),
            executable.path().as_os_str().to_owned(),
        );
    }
    environment.insert("VK_DRIVER_FILES".into(), descriptor.into_os_string());
    Ok(environment)
}

#[cfg(test)]
#[path = "wine_tests.rs"]
mod tests;
