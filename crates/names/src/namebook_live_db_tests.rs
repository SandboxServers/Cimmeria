//! The [`NameBook`] against the seeded database.
//!
//! The gap guard pins every seed row that has no name (blank or a
//! placeholder), by table and id, in `namebook_gaps.txt`. A new blank or
//! placeholder row fails it until the row gets a real name or joins the
//! list; a row that gains a name fails it until it leaves the list, so the
//! list only ever describes the seed as it is. Regenerate the file with
//! `NAMEBOOK_GAPS_BLESS=1` on this test after a deliberate seed change.

use std::collections::{BTreeMap, BTreeSet};

use crate::test_support::require_db_or_skip;
use crate::{
    archetype_name, book, is_placeholder, load_at_boot, query, racial_paradigm_name, NameBook,
    Table, ARCHETYPE_NAMES,
};

const GAPS_FILE: &str = "src/namebook_gaps.txt";
const GAPS: &str = include_str!("namebook_gaps.txt");

type Gaps = BTreeMap<Table, BTreeSet<i64>>;

/// `items 1 4..=9 12`: one table per line, inclusive ranges (ids can be
/// negative, hence `..=`); a table may span several lines. `#` starts a comment.
fn parse_gaps(text: &str) -> Gaps {
    let mut gaps = Gaps::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut words = line.split_whitespace();
        let Some(table) = words.next() else { continue };
        let table = Table::from_name(table).unwrap_or_else(|| panic!("unknown table {table}"));
        let ids = gaps.entry(table).or_default();
        for word in words {
            let (lo, hi) = word.split_once("..=").unwrap_or((word, word));
            let (lo, hi): (i64, i64) = (lo.parse().unwrap(), hi.parse().unwrap());
            ids.extend(lo..=hi);
        }
    }
    gaps
}

fn render_gaps(gaps: &Gaps) -> String {
    let mut out = String::from(
        "# Seed rows with no name (blank or a placeholder), by table and id.\n\
         # Pinned by live_db_namebook_every_seed_row_resolves_or_is_a_pinned_gap;\n\
         # regenerate with NAMEBOOK_GAPS_BLESS=1. Format: `table id lo..=hi ...`.\n",
    );
    for (table, ids) in gaps {
        let mut runs: Vec<String> = Vec::new();
        let mut iter = ids.iter().copied().peekable();
        while let Some(lo) = iter.next() {
            let mut hi = lo;
            while iter.peek() == Some(&(hi + 1)) {
                hi = iter.next().unwrap();
            }
            runs.push(if lo == hi {
                lo.to_string()
            } else {
                format!("{lo}..={hi}")
            });
        }
        for chunk in runs.chunks(12) {
            out.push_str(table.as_str());
            for run in chunk {
                out.push(' ');
                out.push_str(run);
            }
            out.push('\n');
        }
    }
    out
}

async fn seed_ids(pool: &sqlx::PgPool, table: Table) -> Vec<i64> {
    let rows: Vec<(i64, Option<String>)> = sqlx::query_as(query(table))
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| panic!("{}: {e}", table.as_str()));
    rows.into_iter().map(|(id, _)| id).collect()
}

/// Every seeded row in every table resolves to a non-empty, non-placeholder
/// name, except the pinned gaps.
#[tokio::test]
async fn live_db_namebook_every_seed_row_resolves_or_is_a_pinned_gap() {
    let pool = require_db_or_skip!();
    let (book, report) = NameBook::load(&pool).await.expect("load");

    let mut found = Gaps::new();
    for table in Table::ALL {
        let ids = seed_ids(&pool, table).await;
        assert_eq!(ids.len(), report.count(table).rows, "{}", table.as_str());
        for id in ids {
            match book.get(table, id) {
                Some(name) => {
                    assert!(
                        !name.trim().is_empty() && !is_placeholder(name),
                        "{} {id} resolved to {name:?}",
                        table.as_str()
                    );
                }
                None => {
                    found.entry(table).or_default().insert(id);
                }
            }
        }
    }

    if std::env::var_os("NAMEBOOK_GAPS_BLESS").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GAPS_FILE);
        std::fs::write(&path, render_gaps(&found)).expect("write gaps file");
        return;
    }

    let pinned = parse_gaps(GAPS);
    let mut problems = Vec::new();
    for table in Table::ALL {
        let empty = BTreeSet::new();
        let want = pinned.get(&table).unwrap_or(&empty);
        let got = found.get(&table).unwrap_or(&empty);
        let new: Vec<_> = got.difference(want).take(20).collect();
        let healed: Vec<_> = want.difference(got).take(20).collect();
        if !new.is_empty() {
            problems.push(format!("{}: new unnamed rows {new:?}", table.as_str()));
        }
        if !healed.is_empty() {
            problems.push(format!(
                "{}: pinned rows that now have a name {healed:?}",
                table.as_str()
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "the seed's unnamed rows differ from {GAPS_FILE} (name the row, or rerun with \
         NAMEBOOK_GAPS_BLESS=1 and commit the file):\n{}",
        problems.join("\n")
    );
}

/// Known names from the seed, and its placeholders as `None`.
#[tokio::test]
async fn live_db_namebook_resolves_known_seed_names() {
    let pool = require_db_or_skip!();
    let (book, report) = NameBook::load(&pool).await.expect("load");

    assert_eq!(book.world(1), Some("CombatSim"));
    assert_eq!(book.applied_science(1), Some("Biomedical Engineering"));
    assert_eq!(book.error_code(0), Some("CONDITION_FEEDBACK_InvalidEntity"));
    assert_eq!(book.template(64), Some("Thor"));
    assert!(
        book.template_display(64).is_some(),
        "Thor's name_id has a text"
    );
    assert_eq!(
        book.template_display(87),
        None,
        "Demon Jaffa has no name_id"
    );
    assert_eq!(book.dialog(100006), Some("Sandbox greeting dialog"));

    // Mission 1683 is `NO MISSION DISPLAY NAME` in the seed.
    assert_eq!(book.mission(1683), None);
    let placeholder_item: (i64,) = sqlx::query_as(
        "SELECT item_id::bigint FROM resources.items WHERE name = 'NO ITEM NAME' LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("a NO ITEM NAME row");
    assert_eq!(book.item(placeholder_item.0), None);
    assert!(report.count(Table::Items).placeholder > 0);

    // Mission 819 is really called "Unused Explosive"; it is no placeholder.
    assert_eq!(book.mission(819), Some("Unused Explosive"));

    // `spawn_sets` has no seed rows at all; every other table names something.
    for table in Table::ALL {
        if table != Table::SpawnSets {
            assert!(book.len(table) > 0, "{} loaded no names", table.as_str());
        }
    }
    assert_eq!(report.count(Table::SpawnSets).rows, 0);
}

/// The compiled-in archetype names are the seed's, by `EArchetype` ordinal,
/// except `ARCHETYPE_Any`, which the seed leaves blank.
#[tokio::test]
async fn live_db_namebook_archetype_names_match_the_seed() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, String)> = sqlx::query_as(
        "SELECT (array_position(enum_range(NULL::resources.\"EArchetype\"), archetype) - 1)::int, \
                COALESCE(name, '')::text \
         FROM resources.archetypes ORDER BY 1",
    )
    .fetch_all(&pool)
    .await
    .expect("archetypes");
    assert_eq!(rows.len(), ARCHETYPE_NAMES.len());
    for (ordinal, name) in rows {
        if ordinal == 0 {
            assert_eq!(name, "", "ARCHETYPE_Any is blank in the seed");
            assert_eq!(archetype_name(0), Some("Any"));
        } else {
            assert_eq!(
                archetype_name(ordinal),
                Some(name.as_str()),
                "archetype {ordinal}"
            );
        }
    }

    let paradigms: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM resources.racial_paradigm ORDER BY id")
            .fetch_all(&pool)
            .await
            .expect("racial_paradigm");
    for (id, name) in paradigms {
        assert_eq!(
            racial_paradigm_name(id),
            Some(name.as_str()),
            "paradigm {id}"
        );
    }
}

/// The boot load fills the process book and logs `names.loaded`; a second
/// boot call does not load again.
#[tokio::test]
async fn live_db_namebook_load_at_boot_fills_the_global_book_once() {
    let pool = require_db_or_skip!();
    let capture = crate::test_support::LogCapture::install();
    load_at_boot(&pool).await;
    load_at_boot(&pool).await;

    assert_eq!(book().world(1), Some("CombatSim"));
    let loads = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "names" && c.has_field("event", "names.loaded"))
        .count();
    assert_eq!(loads, 1, "the second boot call reads nothing");
}

#[test]
fn gaps_round_trip_through_the_file_format() {
    let mut gaps = Gaps::new();
    gaps.insert(Table::Items, [1, 2, 3, 7, 9, 10].into_iter().collect());
    gaps.insert(
        Table::Texts,
        (-40..40).map(|i| i * 3).chain([-20049]).collect(),
    );
    let text = render_gaps(&gaps);
    assert!(text.contains("items 1..=3 7 9..=10\n"));
    assert_eq!(parse_gaps(&text), gaps);
}
