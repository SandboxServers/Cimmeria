//! `CIMMERIA_LAUNCHER_VERSION`, checked by `build.rs` (which includes this file)
//! and by the shell's unit tests. The updater orders releases by this SemVer
//! (docs/updater.md), so a value it cannot parse must never reach a binary.

/// D-LX15: stable `YYYY.M.(D*100+N)` releases, with 16-bit Windows components.
pub fn validate(version: &str) -> Result<(), String> {
    let parsed = semver::Version::parse(version).map_err(|error| {
        format!("CIMMERIA_LAUNCHER_VERSION={version:?} is not SemVer ({error}); releases use YYYY.M.(D*100+N) (D-LX15)")
    })?;
    if [parsed.major, parsed.minor, parsed.patch]
        .iter()
        .any(|part| *part > u16::MAX as u64)
    {
        return Err(
            "CIMMERIA_LAUNCHER_VERSION exceeds Windows 16-bit file-version components".into(),
        );
    }
    Ok(())
}

/// `TAURI_CONFIG` with `version` set: the JSON tauri-build and
/// `generate_context!` merge over tauri.conf.json. Keys a caller already put in
/// `TAURI_CONFIG` are kept.
pub fn config_patch(existing: Option<&str>, version: &str) -> Result<String, String> {
    let mut patch = match existing {
        Some(json) => serde_json::from_str(json)
            .map_err(|error| format!("TAURI_CONFIG is not JSON ({error})"))?,
        None => serde_json::Value::Object(Default::default()),
    };
    let object = patch
        .as_object_mut()
        .ok_or_else(|| "TAURI_CONFIG is not a JSON object".to_owned())?;
    object.insert("version".into(), version.into());
    Ok(patch.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_versions_are_semver_and_anything_else_fails_the_build() {
        for good in [
            "2026.10.11",
            "2026.1.5",
            "2026.10.11-2",
            "2026.12.3199",
            "65535.65535.65535",
            "0.1.0",
        ] {
            assert_eq!(validate(good), Ok(()), "{good}");
        }
        for bad in [
            "2026.10",
            "2026-10-11",
            "launcher-20261011-abc1234",
            "v2026.10.11",
            "2026.10.011",
            " 2026.10.11",
            "",
        ] {
            let message = validate(bad).expect_err(bad);
            assert!(message.contains("CIMMERIA_LAUNCHER_VERSION"), "{message}");
            assert!(message.contains("D-LX15"), "{message}");
        }
    }

    #[test]
    fn windows_file_version_components_must_not_truncate() {
        for bad in ["65536.1.1", "2026.65536.1", "2026.10.65536"] {
            assert!(validate(bad).unwrap_err().contains("16-bit"));
        }
    }

    #[test]
    fn the_config_patch_sets_the_version_and_keeps_other_keys() {
        assert_eq!(
            config_patch(None, "2026.10.11").unwrap(),
            r#"{"version":"2026.10.11"}"#
        );
        let merged: serde_json::Value = serde_json::from_str(
            &config_patch(
                Some(r#"{"version":"9.9.9","build":{"devUrl":"x"}}"#),
                "2026.10.11",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            merged,
            serde_json::json!({"version": "2026.10.11", "build": {"devUrl": "x"}})
        );
        assert!(config_patch(Some("[1]"), "2026.10.11").is_err());
    }
}
