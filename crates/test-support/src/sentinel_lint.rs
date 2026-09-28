//! The workspace registry of live-DB sentinel ids: every sentinel constant's
//! value is declared in exactly one file (#800).
//!
//! Live-DB tests insert rows keyed by positive `0x7xxx_xxxx` sentinels
//! (TESTING.md, "Sentinel id discipline"). Contributors used to pick a base
//! by reading neighbouring doc comments, which were incomplete, and four
//! modules ended up inserting the same `account` / `sgw_player` ids as
//! another module. CI gives each live-DB test its own database clone, so
//! those collisions only bit a local parallel `cargo test`, but chain ids,
//! effect ids and shard ids had drifted the same way.
//!
//! This scan is the registry: it collects every
//! `const NAME: i32|u32|i64|u64 = 0x…;` in `crates/` whose value lies in
//! `0x7000_0000..0x8000_0000`, and fails when one value is declared in more
//! than one file. Several consts in one file may share a value on purpose
//! (a template id reused as its spawn id). To pick a new base, pick a value
//! this test does not report as taken; decimal offsets from a base
//! (`TEST_BASE + 100`) are not expanded, so keep a module's derived ids
//! inside its own block.

use std::collections::BTreeMap;

use crate::source_scan::{rust_sources, RustSource};

/// The sentinel id space (TESTING.md).
const SENTINEL_RANGE: std::ops::Range<u64> = 0x7000_0000..0x8000_0000;

/// Consts in the range that are not ids, as `(crates_rel, name)`.
///
/// `FIRST_TEST_SENTINEL` is the lower bound of the whole sentinel space,
/// used to exclude test rows from a seed-sequence check; it names no row.
const NOT_IDS: &[(&str, &str)] = &[(
    "cell-catalog/src/cell/spawner/tests/live_db_seed_sequences.rs",
    "FIRST_TEST_SENTINEL",
)];

/// This file quotes const declarations in its own tests; it declares none.
const SELF: &str = "test-support/src/sentinel_lint.rs";

/// One sentinel const declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sentinel {
    file: String,
    line: usize,
    name: String,
    value: u64,
}

/// Parse `0x7000_0500` (underscores allowed).
fn parse_hex(literal: &str) -> Option<u64> {
    let digits = literal.strip_prefix("0x")?.replace('_', "");
    u64::from_str_radix(&digits, 16).ok()
}

/// Every `const NAME: <int> = 0x…;` in `text` whose value is a sentinel.
/// Comment lines are skipped, so doc examples never count.
fn sentinels_in(file: &str, text: &str) -> Vec<Sentinel> {
    let mut out = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim_start();
        if line.starts_with("//") {
            continue;
        }
        let mut rest = line;
        while let Some(pos) = rest.find("const ") {
            // `const` must start a token: not the tail of an identifier.
            let token_start = pos == 0
                || !rest[..pos]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
            let after = &rest[pos + "const ".len()..];
            rest = after;
            if !token_start {
                continue;
            }
            let Some((name, tail)) = after.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let Some((ty, tail)) = tail.split_once('=') else {
                continue;
            };
            if !matches!(ty.trim(), "i32" | "u32" | "i64" | "u64") {
                continue;
            }
            let Some((literal, _)) = tail.split_once(';') else {
                continue;
            };
            let Some(value) = parse_hex(literal.trim()) else {
                continue;
            };
            if SENTINEL_RANGE.contains(&value) {
                out.push(Sentinel {
                    file: file.to_string(),
                    line: idx + 1,
                    name: name.to_string(),
                    value,
                });
            }
        }
    }
    out
}

/// Every sentinel const in the workspace, minus [`NOT_IDS`].
fn workspace_sentinels(sources: &[RustSource]) -> Vec<Sentinel> {
    sources
        .iter()
        .filter(|s| s.crates_rel != SELF)
        .flat_map(|s| sentinels_in(&s.crates_rel, &s.read()))
        .filter(|s| !NOT_IDS.contains(&(s.file.as_str(), s.name.as_str())))
        .collect()
}

/// Values declared in more than one file, with every declaration.
fn duplicates(sentinels: &[Sentinel]) -> BTreeMap<u64, Vec<&Sentinel>> {
    let mut by_value: BTreeMap<u64, Vec<&Sentinel>> = BTreeMap::new();
    for s in sentinels {
        by_value.entry(s.value).or_default().push(s);
    }
    by_value.retain(|_, decls| decls.iter().any(|d| d.file != decls[0].file));
    by_value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_consts_are_unique_across_files() {
        let sentinels = workspace_sentinels(&rust_sources());

        // Non-vacuity: a parser that silently finds nothing must fail.
        assert!(
            sentinels.len() >= 250,
            "found only {} sentinel consts; the scanner is broken",
            sentinels.len()
        );
        assert!(
            sentinels.iter().any(|s| s.file
                == "base-methods/src/base/world_entry/methods/mail/tests/read.rs"
                && s.name == "TEST_BASE"
                && s.value == 0x7000_0500),
            "the mail tests' TEST_BASE = 0x7000_0500 was not found; the scanner is broken"
        );

        let dups = duplicates(&sentinels);
        let report: Vec<String> = dups
            .iter()
            .map(|(value, decls)| {
                let sites: Vec<String> = decls
                    .iter()
                    .map(|d| format!("    crates/{}:{}  {}", d.file, d.line, d.name))
                    .collect();
                format!("{value:#010X}\n{}", sites.join("\n"))
            })
            .collect();
        assert!(
            dups.is_empty(),
            "sentinel values declared in more than one file (TESTING.md, \
             \"Sentinel id discipline\"); move one side to a free block:\n{}",
            report.join("\n")
        );
    }

    #[test]
    fn parser_reads_const_declarations_and_skips_everything_else() {
        let text = [
            "const TEST_BASE: i32 = 0x7000_0500;",
            "    pub(crate) const PLAYER: u32 = 0x7000_A0F1; // trailing",
            "const WIDE: i64 = 0x7005_4001;",
            "/// const DOC: i32 = 0x7000_0600;",
            "// const COMMENTED: i32 = 0x7000_0700;",
            "const LOW: i32 = 0x0000_0500;",
            "const DECIMAL: i32 = 1_879_049_472;",
            "const NAME: &str = \"0x7000_0800\";",
            "const SIGNED: i16 = 0x7000;",
        ]
        .join("\n");
        let got: Vec<(String, usize, u64)> = sentinels_in("a.rs", &text)
            .into_iter()
            .map(|s| (s.name, s.line, s.value))
            .collect();
        assert_eq!(
            got,
            vec![
                ("TEST_BASE".to_string(), 1, 0x7000_0500),
                ("PLAYER".to_string(), 2, 0x7000_A0F1),
                ("WIDE".to_string(), 3, 0x7005_4001),
            ]
        );
    }

    #[test]
    fn a_value_in_two_files_is_reported_and_one_file_reusing_it_is_not() {
        let mut sentinels = sentinels_in(
            "mail.rs",
            "const TEST_BASE: i32 = 0x7000_0500;\nconst ALIAS: i32 = 0x7000_0500;",
        );
        assert!(
            duplicates(&sentinels).is_empty(),
            "one file may reuse its own value"
        );

        sentinels.extend(sentinels_in(
            "resync.rs",
            "const TEST_BASE: i32 = 0x7000_0500;",
        ));
        let dups = duplicates(&sentinels);
        let files: Vec<&str> = dups[&0x7000_0500].iter().map(|s| s.file.as_str()).collect();
        assert_eq!(files, vec!["mail.rs", "mail.rs", "resync.rs"]);
    }
}
