//! Interactive game environment differs intentionally from headless extraction.
use super::*;
use crate::{helper_supervisor::HelperCommand, mac_wine};
use cimmeria_runtime_probe::game_launch::Request;
pub(super) fn prepare(
    plan: &Plan,
    state_root: &Path,
) -> Result<
    (
        HelperCommand,
        Request,
        mac_wine::prerequisites::prefix::Resources,
    ),
    IntentError,
> {
    let runtime = plan.runtime.as_ref().ok_or(StorageError::Corrupt)?;
    let resources = mac_wine::prerequisites::prefix::Resources::reopen(runtime, state_root)?;
    let binaries = preparation::prepare(plan)?;
    let graphics = plan
        .resources
        .graphics
        .as_ref()
        .ok_or(StorageError::Corrupt)?;
    graphics.d3d9.stage(&binaries.join("d3d9.dll"))?;
    let mut environment = mac_wine::environment(&resources.runtime, &resources.prefix)
        .map_err(|_| StorageError::Corrupt)?;
    environment.insert(
        "WINEDLLOVERRIDES".into(),
        "d3d9=n;winemenubuilder.exe,mscoree,mshtml=d".into(),
    );
    environment.insert("CX_FWD_COMPAT_GL_CTX".into(), "1".into());
    if let Some((executable, _)) = &graphics.rosetta_x87 {
        environment.insert(
            "ROSETTA_X87_PATH".into(),
            executable.path().as_os_str().to_owned(),
        );
    }
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
    let spec = HelperCommand {
        executable: resources.runtime.join("bin/wine"),
        arguments: vec![guest(plan.resources.helper.path())?.into_os_string()],
        directory: binaries,
        environment,
    };
    Ok((spec, request, resources))
}
