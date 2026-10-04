//! Preflight names against the Windows filesystem semantics used by the client.
use std::{collections::BTreeMap, path::PathBuf};

use super::{safe_relative, UnpackError};

#[derive(Default)]
pub(super) struct EntryInventory {
    entries: BTreeMap<String, Entry>,
}

struct Entry {
    spelling: String,
    directory: bool,
    explicit: bool,
}

impl EntryInventory {
    /// Directory ancestors may be implicit, then explicitly listed once. Every
    /// spelling must agree, including ancestors on case-sensitive hosts.
    pub(super) fn insert(&mut self, name: &str, directory: bool) -> Result<PathBuf, UnpackError> {
        let invalid = || UnpackError::UnsafePath(name.to_owned());
        if name.starts_with(['/', '\\']) {
            return Err(invalid());
        }
        let parts: Vec<_> = name
            .split(['/', '\\'])
            .filter(|p| !p.is_empty() && *p != ".")
            .collect();
        for part in &parts {
            let stem = part
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            let numbered_device = ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|suffix| {
                    matches!(
                        suffix,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            });
            if *part == ".."
                || part.ends_with(['.', ' '])
                || part
                    .chars()
                    .any(|c| c.is_control() || "<>:\"|?*".contains(c))
                || matches!(
                    stem.as_str(),
                    "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                )
                || numbered_device
            {
                return Err(invalid());
            }
        }
        let relative = safe_relative(name).ok_or_else(invalid)?;
        let mut spelling = String::new();
        for (index, part) in parts.iter().enumerate() {
            if !spelling.is_empty() {
                spelling.push('/');
            }
            spelling.push_str(part);
            let explicit = index + 1 == parts.len();
            let is_directory = !explicit || directory;
            let key = spelling.to_lowercase();
            if let Some(previous) = self.entries.get_mut(&key) {
                if previous.spelling != spelling
                    || !previous.directory
                    || !is_directory
                    || (explicit && previous.explicit)
                {
                    return Err(UnpackError::EntryConflict(name.to_owned()));
                }
                previous.explicit |= explicit;
            } else {
                self.entries.insert(
                    key,
                    Entry {
                        spelling: spelling.clone(),
                        directory: is_directory,
                        explicit,
                    },
                );
            }
        }
        Ok(relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_consistent_shared_and_explicit_directories() {
        let mut index = EntryInventory::default();
        index.insert("Working/binaries/SGW.exe", false).unwrap();
        index.insert("Working/binaries/other.dll", false).unwrap();
        index.insert("Working/", true).unwrap();
        index.insert("Working/binaries/", true).unwrap();
    }

    #[test]
    fn rejects_duplicates_case_aliases_and_file_directory_conflicts_in_both_orders() {
        for (first, second) in [
            ("a.txt", "a.txt"),
            ("A.txt", "a.txt"),
            ("Dir/a.txt", "dir/b.txt"),
            ("dir", "dir/a.txt"),
            ("dir/a.txt", "dir"),
            ("a/b", "a\\b"),
        ] {
            let mut index = EntryInventory::default();
            index.insert(first, false).unwrap();
            assert!(
                matches!(
                    index.insert(second, false),
                    Err(UnpackError::EntryConflict(_))
                ),
                "{first}, {second}"
            );
        }
    }

    #[test]
    fn rejects_rooted_traversal_and_windows_alias_names() {
        for bad in [
            "/a",
            "\\a",
            "../a",
            "C:/a",
            "a:stream",
            "a/NUL.txt",
            "aux",
            "COM1",
            "Lpt³.txt",
            "a.",
            "a ",
            "a?",
            "a\0b",
        ] {
            assert!(
                EntryInventory::default().insert(bad, false).is_err(),
                "{bad:?}"
            );
        }
    }
}
