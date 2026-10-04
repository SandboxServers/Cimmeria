use super::*;
/// Matches the private prefix's explicit Z: -> / mapping. Windows-invalid path
/// spellings are rejected rather than normalized to a different target.
pub(crate) fn guest(path: &Path) -> Result<String, WineError> {
    if !path.is_absolute() {
        return Err(WineError::Invalid);
    }
    let mut parts = Vec::new();
    for part in path.components() {
        match part {
            Component::RootDir => (),
            Component::Normal(name) => {
                let name = name.to_str().ok_or(WineError::Invalid)?;
                let stem = name
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(' ')
                    .to_ascii_uppercase();
                let reserved = matches!(
                    stem.as_str(),
                    "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                ) || ["COM", "LPT"].iter().any(|prefix| {
                    stem.strip_prefix(prefix).is_some_and(|suffix| {
                        matches!(
                            suffix,
                            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                        )
                    })
                });
                if reserved
                    || name.is_empty()
                    || name.ends_with([' ', '.'])
                    || name.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c))
                {
                    return Err(WineError::Invalid);
                }
                parts.push(name);
            }
            _ => return Err(WineError::Invalid),
        }
    }
    Ok(format!("Z:\\{}", parts.join("\\")))
}
