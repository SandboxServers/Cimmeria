//! Read method-index constants out of Rust source files by explicit path.
//!
//! A source scan covers a newly added constant automatically, where a
//! hand-listed `(CONST, "defName")` table would leave it unchecked. The
//! files are named by path from the workspace `crates/` directory, so a file
//! that moves makes the scan fail loudly instead of passing on nothing.

use std::path::PathBuf;

/// One `const NAME: <ty> = <integer literal>;` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Const {
    pub(super) file: String,
    pub(super) line: usize,
    pub(super) name: String,
    pub(super) ty: String,
    pub(super) value: u32,
}

/// `crates/<rel>`.
pub(super) fn crate_file(rel: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).join(rel)
}

/// The contents of `crates/<rel>`; panics if it is missing.
pub(super) fn read(rel: &str) -> String {
    let path = crate_file(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} is gone ({e}); update the def-conformance scan to its new path",
            path.display()
        )
    })
}

/// Every `.rs` file under `crates/<dir>`, except test files, sorted.
pub(super) fn rust_files_under(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_string()];
    while let Some(rel) = stack.pop() {
        let entries = std::fs::read_dir(crate_file(&rel))
            .unwrap_or_else(|e| panic!("crates/{rel} is gone ({e})"));
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let child = format!("{rel}/{name}");
            if entry.path().is_dir() {
                if name != "tests" {
                    stack.push(child);
                }
            } else if name.ends_with(".rs") && name != "tests.rs" && !name.ends_with("_tests.rs") {
                out.push(child);
            }
        }
    }
    out.sort();
    out
}

/// The text of the `{ ... }` block that follows the first `header` in
/// `text` (for `pub mod method_idx {`).
pub(super) fn block<'a>(text: &'a str, header: &str) -> &'a str {
    let start = text
        .find(header)
        .unwrap_or_else(|| panic!("`{header}` not found"));
    let open = start + text[start..].find('{').expect("block has a `{`");
    let mut depth = 0usize;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &text[open..open + i + 1];
                }
            }
            _ => {}
        }
    }
    panic!("`{header}` block is not closed");
}

fn parse_int(literal: &str) -> Option<u32> {
    let literal = literal.replace('_', "");
    match literal.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => literal.parse().ok(),
    }
}

/// Every single-line `const NAME: <ty> = <int>;` in `text` whose type is
/// one of `types`. Comment lines, and consts whose value is a path or an
/// expression, are skipped. `line` counts from `first_line`.
pub(super) fn consts(file: &str, text: &str, types: &[&str], first_line: usize) -> Vec<Const> {
    let mut out = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim_start();
        if line.starts_with("//") {
            continue;
        }
        let Some(pos) = line.find("const ") else {
            continue;
        };
        let rest = &line[pos + "const ".len()..];
        let Some((name, rest)) = rest.split_once(':') else {
            continue;
        };
        let Some((ty, rest)) = rest.split_once('=') else {
            continue;
        };
        let Some((literal, _)) = rest.split_once(';') else {
            continue;
        };
        let (name, ty) = (name.trim(), ty.trim());
        if !types.contains(&ty) {
            continue;
        }
        let Some(value) = parse_int(literal.trim()) else {
            continue;
        };
        out.push(Const {
            file: file.to_string(),
            line: first_line + idx,
            name: name.to_string(),
            ty: ty.to_string(),
            value,
        });
    }
    out
}

/// Consts of `types` in the whole file `crates/<rel>`.
pub(super) fn file_consts(rel: &str, types: &[&str]) -> Vec<Const> {
    consts(rel, &read(rel), types, 1)
}

/// Consts of `types` inside the `header { ... }` block of `crates/<rel>`.
pub(super) fn block_consts(rel: &str, header: &str, types: &[&str]) -> Vec<Const> {
    let text = read(rel);
    let body = block(&text, header);
    let offset = text.find(body).expect("block is a slice of text");
    let first_line = text[..offset].lines().count().max(1);
    consts(rel, body, types, first_line)
}

/// `ON_BEING_NAME_ID_UPDATE` and `onBeingNameIDUpdate` both become
/// `ONBEINGNAMEIDUPDATE`.
pub(super) fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_uppercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consts_reads_typed_integer_literals_only() {
        let text = "pub const A: u16 = 12;\n\
                    pub(crate) const B: u8 = 0xC5;\n\
                    // pub const C: u16 = 3;\n\
                    pub const D: u8 =\n    other::D;\n\
                    const E: u32 = 7;\n\
                    pub const F: u16 = 1_2;";
        let got: Vec<(String, u32, usize)> = consts("f.rs", text, &["u8", "u16"], 10)
            .into_iter()
            .map(|c| (c.name, c.value, c.line))
            .collect();
        assert_eq!(
            got,
            vec![
                ("A".to_string(), 12, 10),
                ("B".to_string(), 0xC5, 11),
                ("F".to_string(), 12, 16),
            ]
        );
    }

    #[test]
    fn block_returns_the_braced_body() {
        let text = "x\npub mod m {\n    const A: u16 = 1;\n    mod n { }\n}\nconst B: u16 = 2;";
        assert_eq!(
            block(text, "pub mod m"),
            "{\n    const A: u16 = 1;\n    mod n { }\n}"
        );
    }

    #[test]
    fn normalize_matches_const_and_def_spellings() {
        assert_eq!(
            normalize("ON_BEING_NAME_ID_UPDATE"),
            normalize("onBeingNameIDUpdate")
        );
        assert_ne!(
            normalize("ON_STATE_FIELD_UPDATE"),
            normalize("onTopSpeedUpdate")
        );
    }
}
