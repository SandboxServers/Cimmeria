//! Cross-file primary-key uniqueness for the per-zone seed splits.
//!
//! `entity_templates*.sql` and `spawnlist*.sql` are split per zone and campaign
//! (Castle rows in `entity_templates.sql`, Tollana in
//! `entity_templates_tollana.sql`, ...). An id inserted by two files only fails
//! at `reload-db` (`entity_templates_pkey`), and a template id doubles as the
//! per-player `add_dialog_set` slot, so a shared id would also put one NPC's
//! quest offers on another. CS-07R hit exactly this with templates 420/421
//! (docs/analysis/castle-continuation/worknotes/CS-07R-id-collision.md).
//!
//! The same guard covers the tables Castle authored into (CS-07bF1, review
//! m-3): `dialogs`, `dialog_sets`, `dialog_set_maps`, `missions`,
//! `content_chains` and `dialog_screens` (keyed by `screen_id`, its second
//! column). Tables that share a seed file with child tables are read per
//! table, because `content_triggers` and friends also start with `chain_id`.

use std::collections::BTreeMap;
use std::path::Path;

const DB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../db/resources");

/// The table named by an `INSERT INTO <table> (` line, if `line` is one.
fn insert_table(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("INSERT INTO ")?;
    rest.split(|c: char| c.is_whitespace() || c == '(').next()
}

/// The `col`-th column (0-based) of every row of `table` (every table when
/// `None`): a single-line `INSERT INTO t (...) VALUES (<id>, ...` or a
/// multi-row continuation `(<id>, ...` under the last `INSERT INTO`. Only
/// numeric ids are returned. Columns before `col` must not contain commas.
fn column_ids(text: &str, table: Option<&str>, col: usize) -> Vec<i64> {
    let mut ids = Vec::new();
    let mut current: Option<&str> = None;
    // A text literal can span lines, and one of its lines may start with (;
    // track quote parity (''  escapes keep it even) and skip those lines.
    let mut in_string = false;
    for line in text.lines() {
        let t = line.trim_start();
        let odd_quotes = t.matches('\'').count() % 2 == 1;
        if in_string {
            in_string ^= odd_quotes;
            continue;
        }
        if !t.starts_with("--") {
            in_string = odd_quotes;
        }
        let rest = if let Some(name) = insert_table(t) {
            current = Some(name);
            match t.find(") VALUES (") {
                Some(i) => &t[i + ") VALUES (".len()..],
                None => continue,
            }
        } else if let Some(r) = t.strip_prefix('(') {
            r
        } else if current.is_some() && t.starts_with("VALUES (") {
            &t["VALUES (".len()..]
        } else {
            continue;
        };
        let wanted = table.is_none() || current == table;
        if wanted {
            if let Some(id) = rest
                .split(',')
                .nth(col)
                .and_then(|s| s.trim().parse::<i64>().ok())
            {
                ids.push(id);
            }
        }
        if t.trim_end().ends_with(';') {
            current = None;
        }
    }
    ids
}

/// The first column of every row, whatever the table.
fn first_column_ids(text: &str) -> Vec<i64> {
    column_ids(text, None, 0)
}

/// Every `<DB>/<dir>/Seed/<prefix>*.sql` as `(file name, text)`.
fn seed_files(dir: &str, prefix: &str) -> Vec<(String, String)> {
    let seed = Path::new(DB).join(dir).join("Seed");
    let mut out = Vec::new();
    let entries =
        std::fs::read_dir(&seed).unwrap_or_else(|e| panic!("read {}: {e}", seed.display()));
    for entry in entries {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with(prefix) && name.ends_with(".sql") {
            out.push((name, std::fs::read_to_string(&path).unwrap()));
        }
    }
    assert!(!out.is_empty(), "no {prefix}*.sql under {}", seed.display());
    out
}

/// Ids of column `col` of `table` inserted more than once across the
/// `(dir, prefix)` sources, with the files that insert them. A repeat inside
/// one file counts. Also returns the number of rows read, so a test can fail
/// loudly when the reader sees only part of the seeds.
fn duplicates_counted(
    sources: &[(&str, &str)],
    table: Option<&str>,
    col: usize,
) -> (BTreeMap<i64, Vec<String>>, usize) {
    let mut seen: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    let mut total = 0;
    for (dir, prefix) in sources {
        for (name, text) in seed_files(dir, prefix) {
            for id in column_ids(&text, table, col) {
                total += 1;
                seen.entry(id).or_default().push(name.clone());
            }
        }
    }
    assert!(total > 0, "no {table:?} rows found in {sources:?}");
    seen.retain(|_, f| f.len() > 1);
    (seen, total)
}

/// The duplicates of [`duplicates_counted`], without the row count.
fn duplicates_in(
    sources: &[(&str, &str)],
    table: Option<&str>,
    col: usize,
) -> BTreeMap<i64, Vec<String>> {
    duplicates_counted(sources, table, col).0
}

/// Ids inserted more than once across `<DB>/<dir>/Seed/<prefix>*.sql`.
fn duplicates(dir: &str, prefix: &str) -> BTreeMap<i64, Vec<String>> {
    duplicates_in(&[(dir, prefix)], None, 0)
}

#[test]
fn entity_template_ids_unique_across_seed_files() {
    let dups = duplicates("Entities", "entity_templates");
    assert!(
        dups.is_empty(),
        "template_id inserted more than once: {dups:?}"
    );
}

#[test]
fn spawn_ids_unique_across_seed_files() {
    let dups = duplicates("Worlds", "spawnlist");
    assert!(
        dups.is_empty(),
        "spawn_id inserted more than once: {dups:?}"
    );
}

#[test]
fn first_column_ids_reads_both_row_shapes() {
    let sql = "INSERT INTO t (a, b) VALUES (420, 'x');\n\
               INSERT INTO t (a, b) VALUES\n\
               (421, 'y'),\n\
               (422, 'z');\n\
               SELECT pg_catalog.setval('s', 1, true);\n";
    assert_eq!(first_column_ids(sql), vec![420, 421, 422]);
}

#[test]
fn column_ids_filters_by_table_and_column() {
    let sql = "INSERT INTO chains (chain_id, d) VALUES (7, 'x');\n\
               INSERT INTO triggers (chain_id, e) VALUES (7, 'x');\n\
               INSERT INTO screens (dialog_id, screen_id, text) VALUES\n\
               (5, 90, 'a'),\n\
               (5, 91, 'b');\n\
               (99, 98, 'c');\n";
    assert_eq!(column_ids(sql, Some("chains"), 0), vec![7]);
    assert_eq!(column_ids(sql, Some("screens"), 1), vec![90, 91]);
    assert_eq!(column_ids(sql, Some("triggers"), 0), vec![7]);
}

#[test]
fn column_ids_skips_lines_inside_a_text_literal() {
    let sql = "INSERT INTO screens (dialog_id, screen_id, text) VALUES (1, 50, 'it''s\n\
               (2, 51, the literal ends here');\n\
               INSERT INTO screens (dialog_id, screen_id, text) VALUES (3, 52, 'x');\n";
    assert_eq!(column_ids(sql, Some("screens"), 1), vec![50, 52]);
}

#[test]
fn column_ids_reads_values_on_the_next_line() {
    let sql = "INSERT INTO chains (chain_id, d)\n\
               VALUES (7, 'x');\n\
               INSERT INTO chains (chain_id, d)\n\
               VALUES (8, 'y');\n";
    assert_eq!(column_ids(sql, Some("chains"), 0), vec![7, 8]);
}

#[test]
fn dialog_ids_unique_across_seed_files() {
    let dups = duplicates_in(&[("Dialogs", "dialogs")], Some("dialogs"), 0);
    assert!(
        dups.is_empty(),
        "dialog_id inserted more than once: {dups:?}"
    );
}

#[test]
fn dialog_set_ids_unique_across_seed_files() {
    let dups = duplicates_in(&[("Dialogs", "dialog_sets")], Some("dialog_sets"), 0);
    assert!(
        dups.is_empty(),
        "dialog_set_id inserted more than once: {dups:?}"
    );
}

#[test]
fn dialog_set_map_ids_unique_across_seed_files() {
    // Harset authored its maps inside the chain seeds.
    let (dups, rows) = duplicates_counted(
        &[("Dialogs", "dialog_set_maps"), ("Content", "")],
        Some("dialog_set_maps"),
        0,
    );
    assert!(
        rows >= 4500,
        "the reader saw only {rows} dialog_set_maps rows; it has lost a row shape"
    );
    // Harset's three maps (120001, 120002, 120101) sit in the next-line
    // `VALUES (` shape, so the row floor alone would not notice losing it.
    let read: Vec<i64> = [("Dialogs", "dialog_set_maps"), ("Content", "")]
        .iter()
        .flat_map(|(dir, prefix)| seed_files(dir, prefix))
        .flat_map(|(_, text)| column_ids(&text, Some("dialog_set_maps"), 0))
        .collect();
    for id in [120001, 120002, 120101] {
        assert!(
            read.contains(&id),
            "dialog_set_map {id} was not read; the next-line VALUES shape is lost"
        );
    }
    assert!(
        dups.is_empty(),
        "dialog_set_map_id inserted more than once: {dups:?}"
    );
}

/// `dialog_screens` is keyed by `screen_id` (column 2); `dialog_id` repeats.
#[test]
fn dialog_screen_ids_unique_across_seed_files() {
    let dups = duplicates_in(&[("Dialogs", "dialog_screens")], Some("dialog_screens"), 1);
    assert!(
        dups.is_empty(),
        "screen_id inserted more than once: {dups:?}"
    );
}

#[test]
fn mission_ids_unique_across_seed_files() {
    let dups = duplicates_in(&[("Missions", "missions")], Some("missions"), 0);
    assert!(
        dups.is_empty(),
        "mission_id inserted more than once: {dups:?}"
    );
}

#[test]
fn content_chain_ids_unique_across_seed_files() {
    let (dups, rows) = duplicates_counted(&[("Content", "")], Some("content_chains"), 0);
    assert!(
        rows >= 450,
        "the reader saw only {rows} content_chains rows; it has lost a row shape"
    );
    assert!(
        dups.is_empty(),
        "chain_id inserted more than once: {dups:?}"
    );
}

/// Castle chain files allocated outside the Castle block before the
/// four-zone plan; `id-allocation.md` records their ranges.
const LEGACY_CASTLE_CHAIN_FILES: [&str; 4] = [
    "castle_701_chains.sql",
    "castle_702_704_chains.sql",
    "castle_706_708_chains.sql",
    "castle_cellblock_chains.sql",
];

/// The Castle block 10000-10999 (`id-allocation.md`, "Castle"), the Castle
/// restoration rows 1367-1400 (1394-1400 is the 567 leg) and the
/// individually approved 1366 (CS-08a, Frost's Letter bridge).
fn in_castle_chain_range(id: i64) -> bool {
    (10000..=10999).contains(&id) || (1366..=1400).contains(&id)
}

/// Every chain a non-legacy Castle seed file authors sits in the Castle
/// ranges, so a stray id cannot take another zone's slot. New
/// `castle_*_chains.sql` files are picked up by the glob.
#[test]
fn castle_chain_ids_stay_in_castle_ranges() {
    let files = seed_files("Content", "castle_");
    let mut checked = 0;
    let mut file_count = 0;
    for (name, text) in &files {
        if !name.ends_with("_chains.sql") || LEGACY_CASTLE_CHAIN_FILES.contains(&name.as_str()) {
            continue;
        }
        file_count += 1;
        let ids = column_ids(text, Some("content_chains"), 0);
        assert!(!ids.is_empty(), "{name} must author chains");
        for id in ids {
            checked += 1;
            assert!(
                in_castle_chain_range(id),
                "{name}: chain {id} is outside 10000-10999, 1366 and 1367-1400"
            );
        }
    }
    assert!(
        file_count >= 7,
        "expected at least 7 Castle chain files, saw {file_count}"
    );
    assert!(checked > 40, "expected the Castle chains, saw {checked}");
}
