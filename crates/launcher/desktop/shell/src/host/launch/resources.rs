use cimmeria_launcher_engine::launch::{Artifact, Graphics, Resources};
use std::path::Path;
pub(super) fn bundled(root: &Path) -> Option<Resources> {
    let artifact = |name: &str, digest: Option<&str>| Artifact::open(root.join(name), digest?).ok();
    let helper = artifact(
        "windows/cimmeria-launch-worker.exe",
        option_env!("CIMMERIA_LAUNCH_HELPER_SHA256"),
    )?;
    // Client patches are part of this product's Play contract, not optional fallback.
    let patches = artifact(
        "windows/cimmeria_client_patches.dll",
        option_env!("CIMMERIA_CLIENT_PATCHES_SHA256"),
    )?;
    let graphics = if cfg!(target_os = "macos") {
        let d3d9 = artifact("graphics/d3d9.dll", option_env!("CIMMERIA_D3D9_SHA256"))?;
        let rosetta_x87 = match (
            option_env!("CIMMERIA_ROSETTA_X87_SHA256"),
            option_env!("CIMMERIA_ROSETTA_X87_LIBRARY_SHA256"),
        ) {
            (None, None) => None,
            (Some(exe), Some(lib)) => Some((
                artifact("graphics/rosettax87", Some(exe))?,
                artifact("graphics/libRuntimeRosettax87", Some(lib))?,
            )),
            _ => return None,
        };
        Some(Graphics { d3d9, rosetta_x87 })
    } else {
        None
    };
    Some(Resources {
        helper,
        client_patches: Some(patches),
        graphics,
    })
}
