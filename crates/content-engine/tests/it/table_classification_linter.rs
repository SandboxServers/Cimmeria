//! Linter for the public-table classification lists under `db/persistent/`.
//!
//! Every table created under `db/sgw/*/Tables/` must appear in exactly one of
//! `reseed-public-tables.txt` (refilled from the seed on every rebuild) or
//! `persistent-public-tables.txt` (carried over on every rebuild). The lint
//! also rejects listed names that have no `CREATE TABLE`, and it checks that
//! the persistence-schema names never leak into the DB source tree.
//!
//! The scan is line-based, the same as `interact_tag_linter`: no DB, no new
//! crates. A `CREATE TABLE` must start its own line to be counted.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Words that must never appear in the DB source tree (the persistence
/// schemas live only under `db/persistent/`).
const FORBIDDEN_WORDS: [&str; 2] = ["cimmeria_meta", "cimmeria_seed"];

/// Resolve the workspace root from the test crate's CARGO_MANIFEST_DIR.
/// `CARGO_MANIFEST_DIR` is `<workspace>/crates/content-engine`, so two
/// `parent()` hops land on the workspace root.
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Parse a classification list: one name per line, `#` starts a comment.
fn parse_list(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let without_comment = match line.find('#') {
                Some(i) => &line[..i],
                None => line,
            };
            let name = without_comment.trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

/// `strip_prefix` that ignores ASCII case, so `create table` is not missed.
fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// Table names created by the `CREATE TABLE` lines of one SQL file.
fn create_table_names(sql: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in sql.lines() {
        let Some(rest) = strip_prefix_ci(line.trim_start(), "CREATE TABLE ") else {
            continue;
        };
        let rest = rest.trim_start();
        let rest = strip_prefix_ci(rest, "IF NOT EXISTS ")
            .unwrap_or(rest)
            .trim_start();
        let token = rest
            .split(|c: char| c.is_whitespace() || c == '(')
            .next()
            .unwrap_or("");
        let token = token.strip_prefix("public.").unwrap_or(token);
        let name = token.replace('"', "").to_lowercase();
        if !name.is_empty() {
            names.push(name);
        }
    }
    names
}

/// Every `*.sql` file directly under `db/sgw/<group>/Tables/`.
fn table_files() -> Vec<PathBuf> {
    let sgw = workspace_root().join("db").join("sgw");
    let mut out = Vec::new();
    for group in fs::read_dir(&sgw).unwrap().flatten() {
        let tables = group.path().join("Tables");
        if !tables.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&tables).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "sql") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Every table name created under `db/sgw/**/Tables`.
fn table_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for path in table_files() {
        let text = fs::read(&path).unwrap();
        names.extend(create_table_names(&String::from_utf8_lossy(&text)));
    }
    names
}

/// Check the classification. Returns one message per violation, in a
/// stable order.
fn lint(tables: &BTreeSet<String>, reseed: &[String], persistent: &[String]) -> Vec<String> {
    let reseed_set: BTreeSet<&str> = reseed.iter().map(String::as_str).collect();
    let persistent_set: BTreeSet<&str> = persistent.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for t in tables {
        match (
            reseed_set.contains(t.as_str()),
            persistent_set.contains(t.as_str()),
        ) {
            (false, false) => out.push(format!("{t} is in neither list")),
            (true, true) => out.push(format!("{t} is in both lists")),
            _ => {}
        }
    }
    let listed: BTreeSet<&str> = reseed_set.union(&persistent_set).copied().collect();
    for name in listed {
        if !tables.contains(name) {
            out.push(format!(
                "{name} is listed but has no CREATE TABLE under db/sgw/**/Tables"
            ));
        }
    }
    for list in [reseed, persistent] {
        let mut seen = BTreeSet::new();
        let mut doubled = BTreeSet::new();
        for name in list {
            if !seen.insert(name.as_str()) {
                doubled.insert(name.as_str());
            }
        }
        for name in doubled {
            out.push(format!("{name} is listed twice"));
        }
    }
    out
}

/// Collect every `*.sql` file under `dir`, recursively.
fn collect_sql(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sql(&path, out);
        } else if path.extension().is_some_and(|e| e == "sql") {
            out.push(path);
        }
    }
}

/// Report every DB source file that names a persistence schema.
fn forbidden_mentions() -> Vec<String> {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_sql(&root.join("db").join("sgw"), &mut files);
    collect_sql(&root.join("db").join("resources"), &mut files);
    let top = root.join("db").join("database.sql");
    if top.is_file() {
        files.push(top);
    }
    let mut out = Vec::new();
    for file in files {
        let text = fs::read(&file).unwrap();
        let lower = String::from_utf8_lossy(&text).to_lowercase();
        let rel = file.strip_prefix(&root).unwrap_or(&file).display();
        for word in FORBIDDEN_WORDS {
            if lower.contains(word) {
                out.push(format!("{rel} mentions {word}"));
            }
        }
    }
    out
}

/// Read both classification lists from `db/persistent/`.
fn real_lists() -> (Vec<String>, Vec<String>) {
    let dir = workspace_root().join("db").join("persistent");
    let reseed = fs::read_to_string(dir.join("reseed-public-tables.txt")).unwrap();
    let persistent = fs::read_to_string(dir.join("persistent-public-tables.txt")).unwrap();
    (parse_list(&reseed), parse_list(&persistent))
}

#[test]
fn real_tables_are_classified_exactly_once() {
    let (reseed, persistent) = real_lists();
    let violations = lint(&table_names(), &reseed, &persistent);
    assert!(
        violations.is_empty(),
        "classification violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn real_lists_have_the_expected_sizes() {
    let (reseed, persistent) = real_lists();
    assert_eq!(persistent.len(), 25, "persistent list size");
    assert!(persistent.iter().any(|n| n == "account"));
    assert_eq!(reseed, vec!["shards".to_string()]);
}

#[test]
fn db_source_never_mentions_the_persistence_schemas() {
    let found = forbidden_mentions();
    assert!(
        found.is_empty(),
        "forbidden mentions:\n{}",
        found.join("\n")
    );
}

#[test]
fn lint_catches_an_unclassified_a_doubled_and_a_stale_table() {
    let tables: BTreeSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    let reseed = vec!["a".to_string()];
    let persistent = vec!["a".to_string(), "c".to_string(), "ghost".to_string()];
    let v = lint(&tables, &reseed, &persistent);
    assert!(
        v.iter().any(|m| m.contains("b is in neither list")),
        "{v:?}"
    );
    assert!(v.iter().any(|m| m.contains("a is in both lists")), "{v:?}");
    assert!(
        v.iter()
            .any(|m| m.contains("ghost is listed but has no CREATE TABLE")),
        "{v:?}"
    );

    let doubled = vec!["c".to_string(), "c".to_string()];
    let v = lint(&tables, &["a".to_string(), "b".to_string()], &doubled);
    assert!(v.iter().any(|m| m.contains("c is listed twice")), "{v:?}");
}

#[test]
fn parse_list_strips_comments_and_blanks() {
    let names = parse_list("# header\n\naccount # trailing\n  shards  \n");
    assert_eq!(names, vec!["account".to_string(), "shards".to_string()]);
}

#[test]
fn create_table_names_handles_prefixes_and_quotes() {
    let sql = "CREATE TABLE public.Sgw_Player (\n  -- CREATE TABLE ignored\n  x int\n);\n\
               CREATE TABLE IF NOT EXISTS \"account\"(id int);\n";
    assert_eq!(
        create_table_names(sql),
        vec!["sgw_player".to_string(), "account".to_string()]
    );
}

#[test]
fn create_table_names_ignores_keyword_case() {
    let sql = "create table sgw_new (id int);\nCreate Table If Not Exists other (id int);\n";
    assert_eq!(
        create_table_names(sql),
        vec!["sgw_new".to_string(), "other".to_string()]
    );
}
