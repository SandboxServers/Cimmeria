//! Lab-only overrides for each instance's user `SGWEngine.ini`.
//!
//! The client uses its user `SGWEngine.ini` as the whole config while its
//! `[IniVersion]` matches the chain files (`docs/client/limits/engine.md`,
//! "Ini chain and lab override targets"). An override file lists the
//! `[section] key = value` lines to set; they are applied to each instance's
//! own seeded copy at every launch, after [`super::instance_profile`] has
//! seeded it, and each applied override is logged under `lab.instance`.
//! Only the named keys change; the rest of the file stays byte-for-byte.

use std::path::{Path, PathBuf};

/// One `[[override]]` entry of `ini-overrides.toml`, checked and trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// An entry as written: `value` may be a TOML string, integer, float or
/// boolean.
#[derive(Debug, serde::Deserialize)]
struct RawOverride {
    file: String,
    section: String,
    key: String,
    value: toml::Value,
}

#[derive(Debug, Default, serde::Deserialize)]
struct OverridesFile {
    #[serde(default, rename = "override")]
    overrides: Vec<RawOverride>,
}

/// The variable naming the overrides file, overriding the search.
pub const OVERRIDES_ENV: &str = "CIMMERIA_LAB_INI_OVERRIDES";

/// The overrides file's name in the lab state dir.
pub const OVERRIDES_FILE: &str = "ini-overrides.toml";

/// The overrides file for this process: see [`overrides_path_from`]. The
/// state dir is the daemon's (`%LOCALAPPDATA%\cimmeria-lab`), which is also
/// the installed daemon's working directory; the walk-up starts from the
/// working directory and from this executable (a checkout run).
pub fn overrides_path() -> Option<PathBuf> {
    let starts: Vec<PathBuf> = [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf)),
    ]
    .into_iter()
    .flatten()
    .collect();
    overrides_path_from(
        std::env::var(OVERRIDES_ENV).ok().as_deref(),
        &crate::daemon::config::state_dir(),
        &starts,
    )
}

/// The first of: `env` (the raw [`OVERRIDES_ENV`] value) when non-empty;
/// `<state_dir>\ini-overrides.toml` when it is a file; the first
/// `tools\lab\ini-overrides.toml` found walking up from each of `starts`.
/// `None` when there is none.
pub fn overrides_path_from(
    env: Option<&str>,
    state_dir: &Path,
    starts: &[PathBuf],
) -> Option<PathBuf> {
    if let Some(p) = env.map(str::trim).filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let in_state = state_dir.join(OVERRIDES_FILE);
    if in_state.is_file() {
        return Some(in_state);
    }
    for start in starts {
        for dir in start.ancestors() {
            let cand = dir.join("tools").join("lab").join(OVERRIDES_FILE);
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

/// Parse the file text. Section and key are trimmed. An entry that fails
/// [`check`], or whose value is not a string, integer, float or boolean,
/// fails the whole file: the caller applies none of it.
pub fn parse(text: &str) -> Result<Vec<IniOverride>, String> {
    let file: OverridesFile =
        toml::from_str(text).map_err(|e| format!("ini-overrides.toml does not parse: {e}"))?;
    file.overrides
        .into_iter()
        .enumerate()
        .map(|(i, raw)| entry(raw).map_err(|why| format!("override {}: {why}", i + 1)))
        .collect()
}

fn entry(raw: RawOverride) -> Result<IniOverride, String> {
    let o = IniOverride {
        value: value_text(&raw.value)?,
        file: raw.file,
        section: raw.section.trim().to_string(),
        key: raw.key.trim().to_string(),
    };
    check(&o).map(|()| o)
}

/// A TOML value as ini text. Booleans are written the way UE3's own ini
/// files spell them, `TRUE` / `FALSE`; floats use Rust's display form.
fn value_text(v: &toml::Value) -> Result<String, String> {
    match v {
        toml::Value::String(s) => Ok(s.clone()),
        toml::Value::Integer(i) => Ok(i.to_string()),
        toml::Value::Float(f) => Ok(f.to_string()),
        toml::Value::Boolean(true) => Ok("TRUE".into()),
        toml::Value::Boolean(false) => Ok("FALSE".into()),
        other => Err(format!(
            "value must be a string, integer, float or boolean, not a {}",
            other.type_str()
        )),
    }
}

/// Device names Windows reserves in every folder, with or without an
/// extension.
const RESERVED_NAMES: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];

/// Whether `file` names a Windows device (`NUL`, `com1.ini`, `LPT9.txt`).
fn is_reserved_name(file: &str) -> bool {
    let stem = file.split('.').next().unwrap_or_default().trim_end();
    if RESERVED_NAMES.iter().any(|r| stem.eq_ignore_ascii_case(r)) {
        return true;
    }
    let (Some(head), Some(tail)) = (stem.get(..3), stem.get(3..)) else {
        return false;
    };
    (head.eq_ignore_ascii_case("COM") || head.eq_ignore_ascii_case("LPT"))
        && matches!(tail.as_bytes(), [b'1'..=b'9'])
}

/// An entry is refused when its `file` is not a plain file name (a path
/// separator, `..`, `:` or a Windows device name), its section or key is
/// empty, its section is `IniVersion` (any case, the stamp the client checks,
/// engine.md) or has a bracket in it, its key has `=`, or any text has a
/// line break.
fn check(o: &IniOverride) -> Result<(), String> {
    if o.file.trim().is_empty()
        || o.file.contains(['/', '\\', ':'])
        || o.file.contains("..")
        || is_reserved_name(&o.file)
    {
        return Err(format!("file {:?} must be a plain name in Config", o.file));
    }
    if o.section.is_empty() {
        return Err("section is empty".into());
    }
    if o.section.eq_ignore_ascii_case("IniVersion") {
        return Err("[IniVersion] is never overridden".into());
    }
    if o.section.contains(['[', ']']) {
        return Err(format!("section {:?} contains a bracket", o.section));
    }
    if o.key.is_empty() || o.key.contains('=') {
        return Err(format!("key {:?} is empty or contains '='", o.key));
    }
    for text in [&o.section, &o.key, &o.value] {
        if text.contains(['\r', '\n']) {
            return Err(format!("{text:?} contains a line break"));
        }
    }
    Ok(())
}

/// Per override, its key and the old value (`None` when the key was absent).
pub type Applied = Vec<(String, Option<String>)>;

/// Apply `overrides` to ini `text`, in order. Returns the new text and, per
/// override, its key and the old value (`None` when the key was absent).
///
/// Section and key match case-insensitively. Only a bare `key=` line
/// matches: `;key=` (a comment) and `+key=` / `-key=` (UE3 array edits)
/// never do. The first match in the section gets the new value; the space
/// after its `=` is kept. A missing key goes after the section's last
/// non-blank line; a missing section is appended with a blank line before
/// it. The file's line ending is CRLF when the text has any CRLF, else LF,
/// and it is used for every inserted line.
pub fn apply(text: &str, overrides: &[IniOverride]) -> (String, Applied) {
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
        let rest = &body[eq + 1..];
        let old = rest.trim().to_string();
        let pad = &rest[..rest.len() - rest.trim_start().len()];
        // A last line with no line break keeps having none.
        let end = if lines[i].ends_with('\n') { nl } else { "" };
        lines[i] = format!("{}{pad}{}{end}", &body[..=eq], o.value);
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

/// Load the overrides from `overrides` (the file [`overrides_path`] found,
/// if any) and apply them to instance `label`'s `config_dir` (its
/// `SGWGame\Config`). Logs and returns on any failure.
pub fn apply_for_instance(config_dir: &Path, label: &str, overrides: Option<&Path>) {
    let Some(path) = overrides else {
        tracing::debug!(target: "lab.instance", event = "ini_override_none", instance = label,
            "no lab ini overrides file ({OVERRIDES_ENV}, the lab state dir, tools/lab)");
        return;
    };
    tracing::info!(target: "lab.instance", event = "ini_override_file", instance = label,
        path = %path.display(), "lab ini overrides file");
    let text = match std::fs::read_to_string(path) {
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
                path = %path.display(), reason = %why, "lab ini overrides not applied");
        }
    }
}

/// [`apply_for_instance`] with the overrides passed in: each file named by
/// them is read from `config_dir`, changed, and written back if the text
/// changed. A missing or unwritable file is logged and skipped.
///
/// Each override logs `ini_override_applied` with `changed` (whether its
/// line was rewritten or added; `false` when the key already had the value).
pub fn apply_overrides(config_dir: &Path, label: &str, overrides: &[IniOverride]) {
    let mut files: Vec<&str> = Vec::new();
    for o in overrides {
        if !files.contains(&o.file.as_str()) {
            files.push(&o.file);
        }
    }
    for file in files {
        let group: Vec<IniOverride> = overrides
            .iter()
            .filter(|o| o.file == file)
            .cloned()
            .collect();
        // Seeds and other launches of this instance may touch the same
        // profile: the read-modify-write runs under the seed lock.
        let result = {
            let _guard = super::instance_profile::seed_lock();
            rewrite(&config_dir.join(file), &group)
        };
        let applied = match result {
            Ok(applied) => applied,
            Err((what, e)) => {
                tracing::warn!(target: "lab.instance", event = "ini_override_failed", instance = label,
                    file = file, reason = %e, "lab ini override target {what}");
                continue;
            }
        };
        for (o, (_, old)) in group.iter().zip(applied) {
            let changed = old.as_deref() != Some(o.value.as_str());
            tracing::info!(target: "lab.instance", event = "ini_override_applied", instance = label,
                file = file, section = %o.section, key = %o.key,
                old = old.as_deref().unwrap_or("<absent>"), new = %o.value, changed,
                "lab ini override applied");
        }
    }
}

/// Apply `group` to the ini at `path` and write it back atomically when its
/// text changed. A leading UTF-8 BOM is set aside while matching and kept.
fn rewrite(path: &Path, group: &[IniOverride]) -> Result<Applied, (&'static str, std::io::Error)> {
    let text = std::fs::read_to_string(path).map_err(|e| ("missing or unreadable", e))?;
    let (bom, body) = match text.strip_prefix('\u{FEFF}') {
        Some(rest) => ("\u{FEFF}", rest),
        None => ("", text.as_str()),
    };
    let (new, applied) = apply(body, group);
    if new != body {
        write_atomic(path, &format!("{bom}{new}")).map_err(|e| ("could not be written", e))?;
    }
    Ok(applied)
}

/// Write `<path>.tmp` beside `path` and rename it over `path`, so a crash
/// mid-write never leaves the client a half ini. A read-only `path` (the
/// seed copies attributes) is made writable first; only that file.
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, text)?;
    let renamed = make_writable(path).and_then(|()| std::fs::rename(&tmp, path));
    if renamed.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    renamed
}

/// Clear `path`'s read-only state, if set: the owner write bit on Unix, the
/// read-only attribute on Windows.
fn make_writable(path: &Path) -> std::io::Result<()> {
    let mut perms = std::fs::metadata(path)?.permissions();
    if !perms.readonly() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(perms.mode() | 0o200);
    }
    #[cfg(not(unix))]
    clear_readonly(&mut perms);
    std::fs::set_permissions(path, perms)
}

/// On Windows `set_readonly(false)` only clears `FILE_ATTRIBUTE_READONLY`;
/// the lint's world-writable concern is Unix-only.
#[cfg(not(unix))]
#[allow(clippy::permissions_set_readonly_false)]
fn clear_readonly(perms: &mut std::fs::Permissions) {
    perms.set_readonly(false);
}

#[cfg(test)]
#[path = "ini_overrides_tests.rs"]
mod tests;
