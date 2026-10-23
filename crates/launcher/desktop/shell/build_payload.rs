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
#[cfg(not(test))]
pub fn generate() -> io::Result<()> {
    println!("cargo:rerun-if-env-changed=CIMMERIA_EMBED_PAYLOAD");
    println!("cargo:rerun-if-env-changed=CIMMERIA_LAUNCHER_TAG");
    for (_, key) in INPUTS {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let required = requires_embedding(
        std::env::var("CIMMERIA_EMBED_PAYLOAD").ok().as_deref(),
        std::env::var("CIMMERIA_LAUNCHER_TAG").ok().as_deref(),
    );
    let pins: Vec<_> = INPUTS
        .iter()
        .map(|(_, key)| std::env::var(key).ok())
        .collect();
    let source = render(
        &Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("resources/windows"),
        windows,
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
    windows: bool,
    required: bool,
    pins: &[Option<String>],
) -> io::Result<String> {
    let mut source = String::from(
        "pub static ENTRIES: &[cimmeria_launcher_engine::embedded_payload::Entry] = &[\n",
    );
    if windows {
        if required && pins.iter().filter(|pin| pin.is_some()).count() != INPUTS.len() {
            return Err(io::Error::other(
                "Windows package requires all three embedded payload pins",
            ));
        }
        for ((name, _), pin) in INPUTS.iter().zip(pins) {
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
    #[test]
    fn tagged_windows_builds_require_all_pins_even_without_the_embed_flag() {
        let root = tempfile::tempdir().unwrap();
        let digest: String = Sha256::digest(b"payload")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let required = requires_embedding(None, Some("launcher-20261023-fixture"));
        for pins in [vec![None; 3], vec![Some(digest.clone()), None, None]] {
            assert!(render(root.path(), true, required, &pins).is_err());
        }
        for (name, _) in INPUTS {
            std::fs::write(root.path().join(name), b"payload").unwrap();
        }
        assert_eq!(
            render(root.path(), true, required, &vec![Some(digest); 3])
                .unwrap()
                .matches("include_bytes!")
                .count(),
            3
        );
        assert!(render(
            root.path(),
            true,
            requires_embedding(None, None),
            &[None, None, None]
        )
        .is_ok());
        assert_eq!(
            render(root.path(), false, required, &[None, None, None])
                .unwrap()
                .matches("include_bytes!")
                .count(),
            0
        );
    }
    #[test]
    fn package_refuses_missing_or_changed_pins_but_mac_does_not_embed_windows_files() {
        let root = tempfile::tempdir().unwrap();
        let pins = vec![None, None, None];
        assert!(render(root.path(), true, true, &pins).is_err());
        assert!(render(root.path(), false, true, &pins)
            .unwrap()
            .ends_with("];\n"));
        let digest: String = Sha256::digest(b"payload")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let pins = vec![Some(digest); 3];
        for (name, _) in INPUTS {
            std::fs::write(root.path().join(name), b"payload").unwrap();
        }
        let source = render(root.path(), true, true, &pins).unwrap();
        assert_eq!(source.matches("include_bytes!").count(), 3);
        std::fs::write(root.path().join(INPUTS[1].0), b"changed").unwrap();
        assert!(render(root.path(), true, true, &pins).is_err());
    }
}
