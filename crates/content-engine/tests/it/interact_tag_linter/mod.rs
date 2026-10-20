//! Linter for content chains that wire `interact_tag` triggers without
//! a matching `set_interaction_type` action. Catches the bug class that
//! made HackTheRings_Switch (chain 1034 in mission 640) and
//! Preparation_RingSwitch (mission 680) silently un-clickable on the
//! client — the chain triggered but no INT_* bit was ever set, so the
//! right-click cursor never registered the interaction.
//!
//! Issue #97 asks for the linter to also walk the entity_templates
//! table for entities whose default flags already include the bit, but
//! template loading needs a real DB. The MVP implementation here scans
//! the seed SQL files directly with a line-by-line pass — for each
//! `interact_tag` trigger on tag T, we require that SOME chain in the
//! same file has a `set_interaction_type` action targeting tag T.
//! The check is intentionally **tag-based across the file** (not
//! same-chain) because content engine missions often split the
//! trigger and the bit-setup across sibling chains in the same scope
//! (e.g., chain 1034 sets the bit, chain 1041 triggers the use). An
//! explicit allowlist of `(file, chain_id)` exceptions covers cases
//! where the bit comes from outside the chain SQL entirely (entity
//! template default, lootable body, etc.).

use allowlist::allowlist;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

// The region-key lint (DK-09b split it out to keep this file under the cap).
mod allowlist;
mod beta_e1;
mod regions;
mod union_omega;

/// Resolve the workspace root from the test crate's CARGO_MANIFEST_DIR.
/// `CARGO_MANIFEST_DIR` is `<workspace>/crates/content-engine`, so two
/// `parent()` hops land on the workspace root. The seed SQL files then
/// live at `<workspace>/db/resources/Content/Seed/`.
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Scan a chain SQL file. Returns:
///   `(interact_tag_chains, tags_with_set_interaction_type_anywhere)`
///
/// where `interact_tag_chains` is `chain_id → npc_tag` (so we can report
/// the failing chain in diagnostics) and the second return is the set of
/// NPC tags that have a `set_interaction_type` action targeting them
/// SOMEWHERE in the file. The check is tag-based, not chain-based —
/// content engine missions often split the trigger and the
/// interaction-flag setup across multiple chains in the same scope.
fn scan_chains(sql: &str) -> (HashMap<i32, String>, HashSet<String>) {
    let mut interact_tag_chains: HashMap<i32, String> = HashMap::new();
    let mut tags_with_sit: HashSet<String> = HashSet::new();

    // We don't try to fully parse SQL — we just look at INSERT VALUES
    // tuples line by line. Format is consistent across the seed files:
    //   (chain_id, 'event_or_action_type', target_id, target_key, ...)
    // The NPC tag for an `interact_tag` trigger is the third quoted
    // string (event_key); for a `set_interaction_type` action it's the
    // fourth (target_key). We collect both.
    for line in sql.lines() {
        let trimmed = line.trim_start();
        let cursor = trimmed.trim_start_matches("VALUES").trim_start();
        if !cursor.starts_with('(') {
            continue;
        }

        let inner = &cursor[1..];
        let chain_id: i32 = match inner
            .split([',', ' '])
            .find(|tok| !tok.is_empty() && tok.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|tok| tok.parse().ok())
        {
            Some(id) => id,
            None => continue,
        };

        // Quoted strings on the line, in order.
        let quoted: Vec<&str> = cursor
            .split('\'')
            .enumerate()
            .filter_map(|(i, s)| if i % 2 == 1 { Some(s) } else { None })
            .collect();

        if cursor.contains("'interact_tag'") {
            // Layout: (chain_id, 'interact_tag', 'NPC_tag', 'scope', ...)
            // Quoted strings: ['interact_tag', 'NPC_tag', 'scope']
            if let Some(npc_tag) = quoted.get(1) {
                interact_tag_chains
                    .entry(chain_id)
                    .or_insert_with(|| npc_tag.to_string());
            }
        }
        if cursor.contains("'set_interaction_type'") {
            // Layout: (chain_id, 'set_interaction_type', target_id, 'NPC_tag', '{params}', ...)
            // Quoted strings: ['set_interaction_type', 'NPC_tag', '{params}']
            if let Some(npc_tag) = quoted.get(1) {
                // Skip the params object (starts with '{').
                if !npc_tag.starts_with('{') {
                    tags_with_sit.insert(npc_tag.to_string());
                }
            }
        }
    }

    (interact_tag_chains, tags_with_sit)
}

#[test]
fn every_interact_tag_chain_has_set_interaction_type() {
    let seed_dir = workspace_root().join("db/resources/Content/Seed");
    assert!(
        seed_dir.exists(),
        "seed dir not found: {}",
        seed_dir.display()
    );

    let mut violations = Vec::new();

    for entry in fs::read_dir(&seed_dir).expect("read seed dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !filename.ends_with("_chains.sql") {
            continue;
        }

        let sql =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let (interact_tag_chains, tags_with_sit) = scan_chains(&sql);

        for (chain_id, npc_tag) in &interact_tag_chains {
            if tags_with_sit.contains(npc_tag) {
                // Same NPC tag has its bit set by some chain in this file.
                continue;
            }
            if allowlist(filename, *chain_id) {
                continue;
            }
            violations.push(format!(
                "  {}:chain {} (interact_tag '{}') has no set_interaction_type action \
                 for tag '{}' anywhere in the file",
                filename, chain_id, npc_tag, npc_tag,
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Chain linter found {} interact_tag chain(s) without a matching \
         set_interaction_type action — clicking the NPC won't register on \
         the client (the INT_* bit is never set):\n{}\n\n\
         Either add a set_interaction_type action to the chain, set the \
         entity template's default flags to include the bit, or add an \
         allowlist entry in tests/it/interact_tag_linter.rs::allowlist with \
         a comment explaining why.",
        violations.len(),
        violations.join("\n"),
    );
}

#[test]
fn scan_chains_picks_up_basic_pattern() {
    // Smoke test the scanner with a tiny synthetic SQL snippet so we
    // catch regressions in the parser independent of the live seed data.
    let sql = r#"
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9999, 'interact_tag', 'TestNPC', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9999, 'set_interaction_type', NULL, 'TestNPC', '{"op":"|","mask":256}', 0, 0);
"#;
    let (triggers, tags) = scan_chains(sql);
    assert!(triggers.contains_key(&9999));
    assert_eq!(triggers.get(&9999).map(String::as_str), Some("TestNPC"));
    assert!(tags.contains("TestNPC"));
}

/// Every `*_chains.sql` in the seed directory must be `\ir`'d from
/// `db/database.sql`.
///
/// This is the one H10-adjacent failure mode neither linter caught: both
/// of them `read_dir` the seed directory, so they pass whether or not a
/// file is registered with the loader. A file that exists but is not in
/// the `\ir` list is never executed by `psql -f db/database.sql`, so
/// every chain in it is simply absent at runtime — and the symptom is not
/// an error, it is content that silently does nothing. CI's `test-live-db`
/// job builds its database from exactly this file.
///
/// Cheap to check and it needs no database: the `\ir` list is plain text.
#[test]
fn every_chain_seed_file_is_registered_in_database_sql() {
    let database_sql_path = workspace_root().join("db/database.sql");
    let database_sql = fs::read_to_string(&database_sql_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", database_sql_path.display()));

    let seed_dir = workspace_root().join("db/resources/Content/Seed");
    let mut seen_any = false;
    let mut missing = Vec::new();

    for entry in fs::read_dir(&seed_dir).expect("read seed dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !filename.ends_with("_chains.sql") {
            continue;
        }
        seen_any = true;
        // The `\ir` paths in database.sql are relative to db/, so the
        // expected line is `\ir resources/Content/Seed/<filename>`.
        let expected = format!("\\ir resources/Content/Seed/{filename}");
        if !database_sql.contains(&expected) {
            missing.push(expected);
        }
    }

    assert!(
        seen_any,
        "found zero *_chains.sql files under {} — parser drift, not an \
         empty seed tree",
        seed_dir.display()
    );
    assert!(
        missing.is_empty(),
        "chain seed file(s) exist but are not loaded by db/database.sql. \
         Every chain in an unregistered file is absent at runtime with no \
         error — the content just never fires. Add the missing line(s), \
         keeping the list's alphabetical order:\n{}",
        missing.join("\n"),
    );
}
