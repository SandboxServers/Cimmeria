//! Lab-only overrides for each instance's user `SGWEngine.ini`.
//!
//! The client uses its user `SGWEngine.ini` as the whole config while its
//! `[IniVersion]` matches the chain files (`docs/client/limits/engine.md`,
//! "Ini chain and lab override targets"). A committed override file lists
//! the `[section] key = value` lines to set; they are applied to each
//! instance's seeded copy at every launch, after [`super::instance_profile`]
//! has seeded it, and each applied override is logged under `lab.instance`.
//! Only the named keys change; the rest of the file stays byte-for-byte.

use std::path::{Path, PathBuf};

/// One `[[override]]` entry of `ini-overrides.toml`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct IniOverride {
    /// The ini file name in the instance's `Config` folder, such as `SGWEngine.ini`.
    pub file: String,
    /// The section name, matched without regard to case.
    pub section: String,
    /// The key, matched without regard to case; its written spelling is kept.
    pub key: String,
    /// The new value, written verbatim after `=`.
    pub value: String,
}

#[derive(Debug, Default, serde::Deserialize)]
struct OverridesFile {
    #[serde(default, rename = "override")]
    overrides: Vec<IniOverride>,
}

/// The variable naming the overrides file, overriding the search.
pub const OVERRIDES_ENV: &str = "CIMMERIA_LAB_INI_OVERRIDES";

/// `CIMMERIA_LAB_INI_OVERRIDES` if set and non-empty, else the first
/// `tools/lab/ini-overrides.toml` found walking up from the working
/// directory and from this executable. `None` when there is none.
pub fn overrides_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var(OVERRIDES_ENV) {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    let starts = [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf)),
    ];
    for start in starts.into_iter().flatten() {
        for dir in start.ancestors() {
            let cand = dir.join("tools").join("lab").join("ini-overrides.toml");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

/// Parse the file text. An entry whose `file` has a path separator or `..`,
/// whose section or key is empty, whose section is `IniVersion` (any case),
/// or whose text has a line break fails the whole file: the caller applies
/// none of it. `[IniVersion]` is the stamp the client checks (engine.md).
pub fn parse(text: &str) -> Result<Vec<IniOverride>, String> {
    let file: OverridesFile =
        toml::from_str(text).map_err(|e| format!("ini-overrides.toml does not parse: {e}"))?;
    for (i, o) in file.overrides.iter().enumerate() {
        check(o).map_err(|why| format!("override {}: {why}", i + 1))?;
    }
    Ok(file.overrides)
}

fn check(o: &IniOverride) -> Result<(), String> {
    if o.file.is_empty() || o.file.contains(['/', '\\']) || o.file.contains("..") {
        return Err(format!("file {:?} must be a plain name in Config", o.file));
    }
    if o.section.trim().is_empty() {
        return Err("section is empty".into());
    }
    if o.section.trim().eq_ignore_ascii_case("IniVersion") {
        return Err("[IniVersion] is never overridden".into());
    }
    if o.key.trim().is_empty() || o.key.contains('=') {
        return Err(format!("key {:?} is empty or contains '='", o.key));
    }
    for text in [&o.section, &o.key, &o.value] {
        if text.contains(['\r', '\n']) {
            return Err(format!("{text:?} contains a line break"));
        }
    }
    Ok(())
}

/// Apply `overrides` to ini `text`, in order. Returns the new text and, per
/// override, its key and the old value (`None` when the key was absent).
///
/// Section and key match case-insensitively. The first `key=` line in the
/// section gets the new value. A missing key goes after the section's last
/// non-blank line; a missing section is appended with a blank line before
/// it. The file's line ending is CRLF when the text has any CRLF, else LF,
/// and it is used for every inserted line.
pub fn apply(text: &str, overrides: &[IniOverride]) -> (String, Vec<(String, Option<String>)>) {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();
    let applied = overrides
        .iter()
        .map(|o| (o.key.clone(), set_key(&mut lines, o, nl)))
        .collect();
    (lines.concat(), applied)
}

/// Set one override in `lines`; returns the old value.
fn set_key(lines: &mut Vec<String>, o: &IniOverride, nl: &str) -> Option<String> {
    let mut in_section = false;
    let mut last = None;
    let mut hit = None;
    for (i, line) in lines.iter().enumerate() {
        let body = line.trim_end_matches(['\r', '\n']);
        if let Some(name) = header_name(body) {
            in_section = name.eq_ignore_ascii_case(&o.section);
            if in_section {
                last = Some(i);
            }
        } else if in_section && !body.trim().is_empty() {
            last = Some(i);
            if hit.is_none() {
                if let Some(eq) = body.find('=') {
                    if body[..eq].trim().eq_ignore_ascii_case(&o.key) {
                        hit = Some((i, eq));
                    }
                }
            }
        }
    }
    if let Some((i, eq)) = hit {
        let body = lines[i].trim_end_matches(['\r', '\n']);
        let old = body[eq + 1..].trim().to_string();
        let new = format!("{}{}{nl}", &body[..=eq], o.value);
        lines[i] = new;
        return Some(old);
    }
    match last {
        Some(i) => {
            if !lines[i].ends_with('\n') {
                lines[i].push_str(nl);
            }
            lines.insert(i + 1, format!("{}={}{nl}", o.key, o.value));
        }
        None => {
            if let Some(last) = lines.last_mut().filter(|l| !l.ends_with('\n')) {
                last.push_str(nl);
            }
            lines.push(format!("{nl}[{}]{nl}{}={}{nl}", o.section, o.key, o.value));
        }
    }
    None
}

/// The name in a `[section]` line, trimmed.
fn header_name(body: &str) -> Option<&str> {
    body.trim()
        .strip_prefix('[')?
        .strip_suffix(']')
        .map(str::trim)
}

/// Load the overrides (if any) and apply them to instance `label`'s
/// `config_dir` (its `SGWGame\Config`). Logs and returns on any failure.
pub fn apply_for_instance(config_dir: &Path, label: &str) {
    let Some(path) = overrides_path() else {
        return;
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(target: "lab.instance", event = "ini_override_failed", instance = label,
                path = %path.display(), reason = %e, "lab ini overrides not applied");
            return;
        }
    };
    match parse(&text) {
        Ok(overrides) => apply_overrides(config_dir, label, &overrides),
        Err(why) => {
            tracing::warn!(target: "lab.instance", event = "ini_override_failed", instance = label,
                reason = %why, "lab ini overrides not applied");
        }
    }
}

/// [`apply_for_instance`] with the overrides passed in: each file named by
/// them is read from `config_dir`, changed, and written back if the text
/// changed. A missing or unwritable file is logged and skipped.
pub fn apply_overrides(config_dir: &Path, label: &str, overrides: &[IniOverride]) {
    let mut files: Vec<&str> = Vec::new();
    for o in overrides {
        if !files.contains(&o.file.as_str()) {
            files.push(&o.file);
        }
    }
    for file in files {
        let path = config_dir.join(file);
        let group: Vec<IniOverride> = overrides
            .iter()
            .filter(|o| o.file == file)
            .cloned()
            .collect();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(target: "lab.instance", event = "ini_override_failed", instance = label,
                    file = file, reason = %e, "lab ini override target missing or unreadable");
                continue;
            }
        };
        let (new, applied) = apply(&text, &group);
        if new != text {
            if let Err(e) = std::fs::write(&path, &new) {
                tracing::warn!(target: "lab.instance", event = "ini_override_failed", instance = label,
                    file = file, reason = %e, "lab ini override could not be written");
                continue;
            }
        }
        for (o, (_, old)) in group.iter().zip(applied) {
            tracing::info!(target: "lab.instance", event = "ini_override_applied", instance = label,
                file = file, section = %o.section, key = %o.key,
                old = old.as_deref().unwrap_or("<absent>"), new = %o.value,
                "lab ini override applied");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ov(file: &str, section: &str, key: &str, value: &str) -> IniOverride {
        IniOverride {
            file: file.into(),
            section: section.into(),
            key: key.into(),
            value: value.into(),
        }
    }

    #[test]
    fn an_override_replaces_the_key_in_its_section_only() {
        // The other section's key comes first: dropping the section match
        // would replace that one instead.
        let text =
            "[Engine.GameEngine]\nHysteresisLimit=20\n\n[TextureStreaming]\nHysteresisLimit=10\n";
        let (out, applied) = apply(
            text,
            &[ov(
                "SGWEngine.ini",
                "TextureStreaming",
                "HysteresisLimit",
                "99",
            )],
        );
        assert_eq!(
            out,
            "[Engine.GameEngine]\nHysteresisLimit=20\n\n[TextureStreaming]\nHysteresisLimit=99\n"
        );
        assert_eq!(
            applied,
            vec![("HysteresisLimit".to_string(), Some("10".to_string()))]
        );
    }

    #[test]
    fn a_missing_key_is_added_to_its_section() {
        let text = "[B]\ny=2\n\n[C]\nq=1\n";
        let (out, applied) = apply(text, &[ov("f.ini", "B", "z", "3")]);
        assert_eq!(out, "[B]\ny=2\nz=3\n\n[C]\nq=1\n");
        assert_eq!(applied, vec![("z".to_string(), None)]);

        // No final line break: one is added before the new line.
        let (out, _) = apply("[B]\ny=2", &[ov("f.ini", "B", "z", "3")]);
        assert_eq!(out, "[B]\ny=2\nz=3\n");
    }

    #[test]
    fn a_missing_section_is_appended() {
        let (out, applied) = apply("[A]\nx=1\n", &[ov("f.ini", "B", "k", "v")]);
        assert_eq!(out, "[A]\nx=1\n\n[B]\nk=v\n");
        assert_eq!(applied, vec![("k".to_string(), None)]);
    }

    #[test]
    fn crlf_is_kept() {
        let text = "[A]\r\nx=1\r\n[B]\r\ny=2\r\n";
        let (out, _) = apply(
            text,
            &[
                ov("f.ini", "A", "z", "3"),
                ov("f.ini", "B", "y", "5"),
                ov("f.ini", "C", "k", "v"),
            ],
        );
        assert_eq!(out, "[A]\r\nx=1\r\nz=3\r\n[B]\r\ny=5\r\n\r\n[C]\r\nk=v\r\n");
        assert!(!out.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn matching_ignores_case() {
        let (out, applied) = apply(
            "[TextureStreaming]\nHysteresisLimit=10\n",
            &[ov("f.ini", "texturestreaming", "hysteresislimit", "7")],
        );
        assert_eq!(out, "[TextureStreaming]\nHysteresisLimit=7\n");
        assert_eq!(
            applied,
            vec![("hysteresislimit".to_string(), Some("10".to_string()))]
        );
    }

    #[test]
    fn iniversion_and_path_escapes_are_refused() {
        let ok =
            "[[override]]\nfile = \"SGWEngine.ini\"\nsection = \"A\"\nkey = \"K\"\nvalue = \"1\"\n";
        assert_eq!(parse(ok).unwrap().len(), 1);
        for text in [
            "[[override]]\nfile = \"SGWEngine.ini\"\nsection = \"IniVersion\"\nkey = \"Stamp\"\nvalue = \"1\"\n",
            "[[override]]\nfile = \"SGWEngine.ini\"\nsection = \"iniversion\"\nkey = \"Stamp\"\nvalue = \"1\"\n",
            "[[override]]\nfile = \"..\\\\x.ini\"\nsection = \"A\"\nkey = \"K\"\nvalue = \"1\"\n",
            "[[override]]\nfile = \"a/b.ini\"\nsection = \"A\"\nkey = \"K\"\nvalue = \"1\"\n",
            "[[override]]\nfile = \"SGWEngine.ini\"\nsection = \"\"\nkey = \"K\"\nvalue = \"1\"\n",
            "[[override]]\nfile = \"SGWEngine.ini\"\nsection = \"A\"\nkey = \"\"\nvalue = \"1\"\n",
            "[[override]]\nfile = \"SGWEngine.ini\"\nsection = \"A\"\nkey = \"K\"\nvalue = \"1\\n[B]\"\n",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn overrides_apply_again_after_a_reseed() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("SGWEngine.ini");
        let stock = "[TextureStreaming]\nHysteresisLimit=10\n";
        let o = [ov(
            "SGWEngine.ini",
            "TextureStreaming",
            "HysteresisLimit",
            "99",
        )];

        std::fs::write(&path, stock).unwrap();
        apply_overrides(tmp.path(), "p2", &o);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[TextureStreaming]\nHysteresisLimit=99\n"
        );

        // A reseed copies the stock file back over the override.
        std::fs::write(&path, stock).unwrap();
        apply_overrides(tmp.path(), "p2", &o);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[TextureStreaming]\nHysteresisLimit=99\n"
        );
    }

    #[test]
    fn a_missing_target_file_is_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        apply_overrides(
            tmp.path(),
            "p2",
            &[ov(
                "SGWEngine.ini",
                "TextureStreaming",
                "HysteresisLimit",
                "99",
            )],
        );
        assert!(!tmp.path().join("SGWEngine.ini").exists());
    }

    #[test]
    fn the_example_file_parses_to_nothing() {
        let example = include_str!("../../../../tools/lab/ini-overrides.example.toml");
        assert_eq!(parse(example).unwrap(), Vec::new());
    }
}
