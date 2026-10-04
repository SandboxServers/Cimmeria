use cimmeria_launcher_engine::launch::{Artifact, Graphics, Resources};
use std::path::Path;
/// Expected identities come from the trusted build, never from the bundle.
pub(super) struct Digests<'a> {
    pub helper: Option<&'a str>,
    pub client_patches: Option<&'a str>,
    pub d3d9: Option<&'a str>,
    pub rosetta_x87: Option<&'a str>,
    pub rosetta_x87_library: Option<&'a str>,
}
pub(super) fn bundled(root: &Path) -> Option<Resources> {
    bundled_with(
        root,
        &Digests {
            helper: option_env!("CIMMERIA_LAUNCH_HELPER_SHA256"),
            client_patches: option_env!("CIMMERIA_CLIENT_PATCHES_SHA256"),
            d3d9: option_env!("CIMMERIA_D3D9_SHA256"),
            rosetta_x87: option_env!("CIMMERIA_ROSETTA_X87_SHA256"),
            rosetta_x87_library: option_env!("CIMMERIA_ROSETTA_X87_LIBRARY_SHA256"),
        },
    )
}
pub(super) fn bundled_with(root: &Path, digests: &Digests) -> Option<Resources> {
    let artifact = |name: &str, digest: Option<&str>| Artifact::open(root.join(name), digest?).ok();
    let helper = artifact("windows/cimmeria-launch-worker.exe", digests.helper)?;
    // Client patches stay part of this product's Play contract, but that policy is
    // decided per installation at admission: an absent or replaced artifact only
    // rules out the installations that inject it.
    let client_patches = artifact(
        "windows/cimmeria_client_patches.dll",
        digests.client_patches,
    );
    let graphics = if cfg!(target_os = "macos") {
        let d3d9 = artifact("graphics/d3d9.dll", digests.d3d9)?;
        let rosetta_x87 = match (digests.rosetta_x87, digests.rosetta_x87_library) {
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
        client_patches,
        graphics,
    })
}
// Fixture paths join with `/` under a canonical root, which Windows verbatim
// paths do not accept; the same code is compiled on both hosts.
#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    fn digest(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    #[test]
    fn absent_or_replaced_patch_artifact_leaves_the_rest_of_the_bundle_usable() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        for directory in ["windows", "graphics"] {
            std::fs::create_dir(root.join(directory)).unwrap();
        }
        std::fs::write(root.join("windows/cimmeria-launch-worker.exe"), b"helper").unwrap();
        std::fs::write(root.join("graphics/d3d9.dll"), b"d3d9").unwrap();
        let (helper, patches, d3d9) = (digest(b"helper"), digest(b"patches"), digest(b"d3d9"));
        let digests = |client_patches| Digests {
            helper: Some(&helper),
            client_patches,
            d3d9: Some(&d3d9),
            rosetta_x87: None,
            rosetta_x87_library: None,
        };
        let dll = root.join("windows/cimmeria_client_patches.dll");
        // Absent file, then a build that pins no patch artifact at all.
        for pinned in [Some(patches.as_str()), None] {
            let bundle = bundled_with(&root, &digests(pinned)).expect("bundle without patches");
            assert!(bundle.client_patches.is_none());
            assert_eq!(bundle.graphics.is_some(), cfg!(target_os = "macos"));
            bundle.verify().unwrap();
        }
        std::fs::write(&dll, b"patches").unwrap();
        let bundle = bundled_with(&root, &digests(Some(&patches))).unwrap();
        assert_eq!(bundle.client_patches.as_ref().unwrap().path(), dll);
        // A replaced DLL is never trusted, and still does not take the helper down.
        std::fs::write(&dll, b"replacement").unwrap();
        let bundle = bundled_with(&root, &digests(Some(&patches))).unwrap();
        assert!(bundle.client_patches.is_none());
        // The launch helper itself stays mandatory for every installation.
        std::fs::write(root.join("windows/cimmeria-launch-worker.exe"), b"other").unwrap();
        assert!(bundled_with(&root, &digests(Some(&patches))).is_none());
    }
}
