//! Zip entry names the Windows swap accepts. Every name is checked before the
//! stage exists, so a name Windows would resolve somewhere unexpected (a
//! device, an alias, a parent folder) never reaches disk.
use super::{
    zip_swap::{refuse, STAGE_PREFIX},
    Error,
};

/// The `/`-separated parts of an entry name, or a refusal.
pub(super) fn components(name: &str, dir: bool) -> Result<Vec<String>, Error> {
    let trimmed = if dir {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    let parts: Vec<&str> = trimmed.split('/').collect();
    let reserved = |part: &str| {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        let port = stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .map(|rest| {
                let mut chars = rest.chars();
                // Windows also reserves the superscript digits `¹²³`.
                chars
                    .next()
                    .is_some_and(|c| c.is_ascii_digit() || matches!(c, '¹' | '²' | '³'))
                    && chars.next().is_none()
            });
        matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || port == Some(true)
    };
    // `~` then a digit reads as an 8.3 short name (`LAUNCH~1.EXE`), which can
    // alias a different long name already on disk.
    let short_name = |part: &str| {
        part.as_bytes()
            .windows(2)
            .any(|pair| pair[0] == b'~' && pair[1].is_ascii_digit())
    };
    // `..`, absolute paths, drive letters, backslash separators, Windows
    // aliases (trailing dots and spaces, 8.3 short names) and device names
    // never reach disk.
    if trimmed.is_empty()
        || parts.iter().any(|part| {
            part.is_empty()
                || *part == "."
                || *part == ".."
                || part.ends_with('.')
                || part.ends_with(' ')
                || part
                    .chars()
                    .any(|c| c.is_control() || r#"\:*?"<>|"#.contains(c))
                || reserved(part)
                || short_name(part)
        })
    {
        return Err(refuse("unsafe_path", name));
    }
    if parts[0].to_lowercase().starts_with(STAGE_PREFIX) {
        return Err(refuse("reserved_name", name));
    }
    Ok(parts.into_iter().map(str::to_owned).collect())
}
