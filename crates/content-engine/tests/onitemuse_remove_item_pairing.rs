//! Regression lint for `item_use` / `remove_item` pairing in seed chains.
//!
//! `handle_use_inventory_item` fires `OnItemUse` without consuming the stack;
//! consumable chains must include an explicit `remove_item` action, while
//! reusable tools (radios, worn equipment, disguises) must omit it.
//!
//! Issue #332. See `docs/content/consumable-via-onitemuse-pattern.md`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// Item design ids expected to be **consumed** on use — every `item_use`
/// chain for these ids must include a `remove_item` action in the same chain.
const KNOWN_CONSUMABLES: &[i32] = &[19, 2893];

/// Item design ids expected to be **reused** on use — every `item_use`
/// chain for these ids must NOT include a `remove_item` action.
const KNOWN_REUSABLES: &[i32] = &[3438, 5168, 2819];

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemUseChain {
    chain_id: i32,
    item_id: i32,
}

/// Scan a chain SQL file for `item_use` triggers.
/// Returns `chain_id → item_id` (event_key parsed as i32).
fn scan_item_use_triggers(sql: &str) -> HashMap<i32, i32> {
    let mut out = HashMap::new();
    for line in sql.lines() {
        let trimmed = line.trim_start();
        let cursor = trimmed.trim_start_matches("VALUES").trim_start();
        if !cursor.starts_with('(') || !cursor.contains("'item_use'") {
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

        let quoted: Vec<&str> = cursor
            .split('\'')
            .enumerate()
            .filter_map(|(i, s)| if i % 2 == 1 { Some(s) } else { None })
            .collect();

        // Layout: (chain_id, 'item_use', '<item_id>', 'scope', ...)
        if let Some(item_key) = quoted.get(1).and_then(|s| s.parse::<i32>().ok()) {
            out.insert(chain_id, item_key);
        }
    }
    out
}

/// Scan a chain SQL file for chains that include a `remove_item` action.
fn scan_chains_with_remove_item(sql: &str) -> HashSet<i32> {
    let mut out = HashSet::new();
    for line in sql.lines() {
        let trimmed = line.trim_start();
        let cursor = trimmed.trim_start_matches("VALUES").trim_start();
        if !cursor.starts_with('(') || !cursor.contains("'remove_item'") {
            continue;
        }

        let inner = &cursor[1..];
        if let Some(chain_id) = inner
            .split([',', ' '])
            .find(|tok| !tok.is_empty() && tok.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|tok| tok.parse().ok())
        {
            out.insert(chain_id);
        }
    }
    out
}

fn known_consumables() -> HashSet<i32> {
    KNOWN_CONSUMABLES.iter().copied().collect()
}

fn known_reusables() -> HashSet<i32> {
    KNOWN_REUSABLES.iter().copied().collect()
}

/// Validate pairing for one seed file. Returns human-readable violation lines.
fn pairing_violations(filename: &str, sql: &str) -> Vec<String> {
    let triggers = scan_item_use_triggers(sql);
    let with_remove = scan_chains_with_remove_item(sql);
    let consumables = known_consumables();
    let reusables = known_reusables();

    let mut violations = Vec::new();
    for (chain_id, item_id) in triggers {
        let has_remove = with_remove.contains(&chain_id);
        let chain = ItemUseChain { chain_id, item_id };

        if consumables.contains(&item_id) {
            if !has_remove {
                violations.push(format!(
                    "  {filename}: chain {chain_id} (item_use item {item_id}) is a \
                     KNOWN_CONSUMABLE but has no remove_item action — player can use \
                     the item infinitely"
                ));
            }
            continue;
        }

        if reusables.contains(&item_id) {
            if has_remove {
                violations.push(format!(
                    "  {filename}: chain {chain_id} (item_use item {item_id}) is a \
                     KNOWN_REUSABLE but has remove_item — likely a copy-paste mistake"
                ));
            }
            continue;
        }

        violations.push(format!(
            "  {filename}: chain {chain_id} (item_use item {item_id}) is not in \
             KNOWN_CONSUMABLES or KNOWN_REUSABLES — add to one of the two lists \
             (tests/onitemuse_remove_item_pairing.rs) and document the intent in \
             docs/content/consumable-via-onitemuse-pattern.md"
        ));

        let _ = chain; // keep struct for mutation-test clarity
    }
    violations
}

#[test]
fn every_item_use_chain_has_correct_remove_item_pairing() {
    let seed_dir = workspace_root().join("db/resources/Content/Seed");
    assert!(
        seed_dir.exists(),
        "seed dir not found: {}",
        seed_dir.display()
    );

    let mut violations = Vec::new();
    let mut seen_any = false;

    for entry in fs::read_dir(&seed_dir).expect("read seed dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !filename.ends_with("_chains.sql") {
            continue;
        }

        let sql =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let file_violations = pairing_violations(filename, &sql);
        if !scan_item_use_triggers(&sql).is_empty() {
            seen_any = true;
        }
        violations.extend(file_violations);
    }

    assert!(
        seen_any,
        "found zero item_use triggers in seed — parser drift or empty tree"
    );
    assert!(
        violations.is_empty(),
        "OnItemUse / RemoveItem pairing lint found {} violation(s):\n{}\n\n\
         See docs/content/consumable-via-onitemuse-pattern.md",
        violations.len(),
        violations.join("\n"),
    );
}

#[test]
fn scan_item_use_triggers_extracts_chain_and_item() {
    let sql = r#"
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1034, 'item_use', '19', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3021, 'item_use', '5168', 'player', false, 0);
"#;
    let triggers = scan_item_use_triggers(sql);
    assert_eq!(triggers.get(&1034), Some(&19));
    assert_eq!(triggers.get(&3021), Some(&5168));
}

#[test]
fn mutation_missing_remove_item_on_consumable_is_caught() {
    // Ambernol chain 1034 shape without remove_item — must fail the lint.
    let sql = r#"
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1034, 'item_use', '19', 'player', false, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1034, 'launch_ability', 1374, NULL, '{}', 0, 0),
  (1034, 'complete_mission', 639, NULL, '{}', 0, 1);
"#;
    let violations = pairing_violations("synthetic.sql", sql);
    assert!(
        violations
            .iter()
            .any(|v| v.contains("chain 1034") && v.contains("KNOWN_CONSUMABLE")),
        "deleting remove_item from the Ambernol chain must trip the guard; got: {violations:?}"
    );
}

#[test]
fn mutation_remove_item_on_reusable_is_caught() {
    // Radio chain 3021 with a spurious remove_item — must fail the lint.
    let sql = r#"
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3021, 'item_use', '5168', 'player', false, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3021, 'display_dialog', 5363, NULL, '{}', 0, 0),
  (3021, 'remove_item', 5168, NULL, '{"qty": 1}', 0, 1);
"#;
    let violations = pairing_violations("synthetic.sql", sql);
    assert!(
        violations
            .iter()
            .any(|v| v.contains("chain 3021") && v.contains("KNOWN_REUSABLE")),
        "adding remove_item to the radio reusable chain must trip the guard; got: {violations:?}"
    );
}

#[test]
fn unknown_item_id_requires_allowlist_entry() {
    let sql = r#"
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9999, 'item_use', '4242', 'player', false, 0);
"#;
    let violations = pairing_violations("synthetic.sql", sql);
    assert_eq!(violations.len(), 1);
    assert!(
        violations[0].contains("add to one of the two lists"),
        "unknown item must force an author decision; got: {:?}",
        violations[0]
    );
}
