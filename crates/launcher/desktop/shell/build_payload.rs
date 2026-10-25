//! Generate embedded Windows resources from independently pinned staging files.
use sha2::{Digest, Sha256};
use std::{io, path::Path};
const INPUTS: [(&str, &str); 3] = [
    (
        "cimmeria-launch-worker.exe",
        "CIMMERIA_LAUNCH_HELPER_SHA256",
    ),
    (
        "cimmeria_client_patches.dll",
        "CIMMERIA_CLIENT_PATCHES_SHA256",
    ),
    (
        "cimmeria_client_telemetry.dll",
        "CIMMERIA_CLIENT_TELEMETRY_SHA256",
    ),
];
/// The prerequisite worker Linux Play runs under Proton, pinned like the others.
const PREREQUISITE: (&str, &str) = (
    "cimmeria-prerequisite-worker.exe",
    "CIMMERIA_PREREQUISITE_HELPER_SHA256",
);
/// The pinned inputs for a target: Linux embeds the prerequisite worker too.
fn inputs(target_os: &str) -> Vec<(&'static str, &'static str)> {
    let mut inputs = INPUTS.to_vec();
    if target_os == "linux" {
        inputs.push(PREREQUISITE);
    }
    inputs
}
/// Whether a build for this target embeds the Windows pieces.
fn embeds_payload(target_os: &str) -> bool {
    matches!(target_os, "windows" | "linux")
}
#[cfg(not(test))]
pub fn generate() -> io::Result<()> {
    println!("cargo:rerun-if-env-changed=CIMMERIA_EMBED_PAYLOAD");
    println!("cargo:rerun-if-env-changed=CIMMERIA_LAUNCHER_TAG");
    for (_, key) in INPUTS.iter().chain(std::iter::once(&PREREQUISITE)) {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let embeds = embeds_payload(&target);
    let inputs = inputs(&target);
    let required = requires_embedding(
        std::env::var("CIMMERIA_EMBED_PAYLOAD").ok().as_deref(),
        std::env::var("CIMMERIA_LAUNCHER_TAG").ok().as_deref(),
    );
    let pins: Vec<_> = inputs
        .iter()
        .map(|(_, key)| std::env::var(key).ok())
        .collect();
    let source = render(
        &Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("resources/windows"),
        &inputs,
        embeds,
        required,
        &pins,
    )?;
    std::fs::write(
        Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("embedded_payload.rs"),
        source,
    )
}
fn requires_embedding(flag: Option<&str>, tag: Option<&str>) -> bool {
    flag == Some("1") || tag.is_some_and(|tag| !tag.trim().is_empty())
}
fn render(
    staged: &Path,
    inputs: &[(&str, &str)],
    embeds: bool,
    required: bool,
    pins: &[Option<String>],
) -> io::Result<String> {
    let mut source = String::from(
        "pub static ENTRIES: &[cimmeria_launcher_engine::embedded_payload::Entry] = &[\n",
    );
    if embeds {
        if required && pins.iter().filter(|pin| pin.is_some()).count() != inputs.len() {
            return Err(io::Error::other(
                "package requires every embedded payload pin",
            ));
        }
        for ((name, _), pin) in inputs.iter().zip(pins) {
            let Some(pin) = pin else {
                continue;
            };
            let path = staged.join(name);
            println!("cargo:rerun-if-changed={}", path.display());
            let bytes = std::fs::read(&path)?;
            let actual: String = Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            if actual != *pin {
                return Err(io::Error::other(format!(
                    "embedded payload pin mismatch: {name}"
                )));
            }
            let path = path.canonicalize()?;
            source.push_str(&format!("cimmeria_launcher_engine::embedded_payload::Entry {{ name: {name:?}, sha256: {pin:?}, bytes: include_bytes!({:?}) }},\n", path.to_str().ok_or_else(|| io::Error::other("non-UTF8 staging path"))?));
        }
    }
    source.push_str("];\n");
    Ok(source)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn digest_of(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    #[test]
    fn tagged_windows_builds_require_all_pins_even_without_the_embed_flag() {
        let root = tempfile::tempdir().unwrap();
        let digest = digest_of(b"payload");
        let windows = inputs("windows");
        let required = requires_embedding(None, Some("launcher-20261023-fixture"));
        for pins in [vec![None; 3], vec![Some(digest.clone()), None, None]] {
            assert!(render(root.path(), &windows, true, required, &pins).is_err());
        }
        for (name, _) in INPUTS {
            std::fs::write(root.path().join(name), b"payload").unwrap();
        }
        assert_eq!(
            render(
                root.path(),
                &windows,
                true,
                required,
                &vec![Some(digest); 3]
            )
            .unwrap()
            .matches("include_bytes!")
            .count(),
            3
        );
        assert!(render(
            root.path(),
            &windows,
            true,
            requires_embedding(None, None),
            &[None, None, None]
        )
        .is_ok());
        assert_eq!(
            render(root.path(), &windows, false, required, &[None, None, None])
                .unwrap()
                .matches("include_bytes!")
                .count(),
            0
        );
    }
    #[test]
    fn package_refuses_missing_or_changed_pins_but_mac_does_not_embed_windows_files() {
        let root = tempfile::tempdir().unwrap();
        let windows = inputs("windows");
        let pins = vec![None, None, None];
        assert!(render(root.path(), &windows, true, true, &pins).is_err());
        assert!(render(root.path(), &windows, false, true, &pins)
            .unwrap()
            .ends_with("];\n"));
        let pins = vec![Some(digest_of(b"payload")); 3];
        for (name, _) in INPUTS {
            std::fs::write(root.path().join(name), b"payload").unwrap();
        }
        let source = render(root.path(), &windows, true, true, &pins).unwrap();
        assert_eq!(source.matches("include_bytes!").count(), 3);
        std::fs::write(root.path().join(INPUTS[1].0), b"changed").unwrap();
        assert!(render(root.path(), &windows, true, true, &pins).is_err());
    }
    #[test]
    fn linux_payload_refuses_a_mismatched_prerequisite_pin_before_embedding() {
        let root = tempfile::tempdir().unwrap();
        let linux = inputs("linux");
        assert_eq!(linux.len(), 4);
        assert_eq!(linux[3], PREREQUISITE);
        let pins = vec![Some(digest_of(b"payload")); 4];
        for (name, _) in &linux {
            std::fs::write(root.path().join(name), b"payload").unwrap();
        }
        std::fs::write(root.path().join(PREREQUISITE.0), b"tampered").unwrap();
        // Nothing is embedded, so no entry can be extracted from this build.
        assert!(render(root.path(), &linux, true, true, &pins).is_err());
        std::fs::write(root.path().join(PREREQUISITE.0), b"payload").unwrap();
        let source = render(root.path(), &linux, true, true, &pins).unwrap();
        assert_eq!(source.matches("include_bytes!").count(), 4);
        // Windows keeps its three entries and never embeds the Linux worker.
        assert_eq!(inputs("windows").len(), 3);
    }
    #[test]
    fn linux_embedding_rules() {
        assert!(
            embeds_payload("linux")
                && embeds_payload("windows")
                && !embeds_payload("macos")
                && !embeds_payload("")
        );
        let root = tempfile::tempdir().unwrap();
        let linux = inputs("linux");
        for (name, _) in &linux {
            std::fs::write(root.path().join(name), b"payload").unwrap();
        }
        // A tagged Linux build with the worker pin absent is refused.
        let mut missing = vec![Some(digest_of(b"payload")); 4];
        missing[3] = None;
        assert!(render(root.path(), &linux, true, true, &missing).is_err());
        // A developer Linux build with no pins and no flag builds with no payload.
        let dev = render(root.path(), &linux, true, false, &[None, None, None, None]).unwrap();
        assert_eq!(dev.matches("include_bytes!").count(), 0);
    }
}
