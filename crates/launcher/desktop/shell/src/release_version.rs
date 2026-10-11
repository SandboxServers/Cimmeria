//! `CIMMERIA_LAUNCHER_VERSION`, checked by `build.rs` (which includes this file)
//! and by the shell's unit tests. The updater orders releases by this SemVer
//! (docs/updater.md), so a value it cannot parse must never reach a binary.

/// D-LX8: `YYYY.M.D` from the release date, `YYYY.M.D-N` for a same-day rebuild.
pub fn validate(version: &str) -> Result<(), String> {
    semver::Version::parse(version).map(drop).map_err(|error| {
        format!(
            "CIMMERIA_LAUNCHER_VERSION={version:?} is not SemVer ({error}). \
             A release is YYYY.M.D, or YYYY.M.D-N for a same-day rebuild (D-LX8)."
        )
    })
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
        for good in ["2026.10.11", "2026.1.5", "2026.10.11-2", "0.1.0"] {
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
            assert!(message.contains("YYYY.M.D"), "{message}");
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
