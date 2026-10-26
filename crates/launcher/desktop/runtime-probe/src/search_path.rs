//! Wine's LdrAddDllDirectory takes `X:\…` and rooted paths only and refuses
//! the `\\?\X:\…` form that `canonicalize` returns (PP-S2, F-24). Windows takes both.
use std::borrow::Cow;

const VERBATIM: &str = r"\\?\";
/// Win32 MAX_PATH. A plain path this long needs the verbatim prefix to be
/// usable at all, so it is passed through unchanged.
const MAX_PATH: usize = 260;

/// The plain drive form of a verbatim drive path, for `AddDllDirectory`.
pub fn dll_search_directory(path: &str) -> Cow<'_, str> {
    let Some(rest) = path.strip_prefix(VERBATIM) else {
        return Cow::Borrowed(path);
    };
    let bytes = rest.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'\\' {
        return Cow::Borrowed(path);
    }
    if rest.contains('/') {
        return Cow::Borrowed(path);
    }
    // Win32 trims trailing dots and spaces and resolves `.`/`..` and empty
    // components for plain paths, but not for verbatim ones, so the two forms
    // would name different folders.
    let components_ok = rest.len() == 3
        || rest[3..].split('\\').all(|c| {
            !c.is_empty() && c != "." && c != ".." && !c.ends_with('.') && !c.ends_with(' ')
        });
    if !components_ok || rest.encode_utf16().count() >= MAX_PATH {
        return Cow::Borrowed(path);
    }
    Cow::Borrowed(rest)
}

#[cfg(test)]
#[path = "search_path_tests.rs"]
mod tests;
