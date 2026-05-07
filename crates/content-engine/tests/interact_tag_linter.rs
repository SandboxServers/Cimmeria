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

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

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

/// Per-file allowlist of `chain_id` values that legitimately have an
/// `interact_tag` trigger without a `set_interaction_type` action in the
/// same chain — typically because the bit comes from the entity template
/// default or another chain in the same mission scope.
///
/// Adding to this list is a deliberate authoring decision; new entries
/// should be justified inline. If you find yourself adding many at once,
/// the entity-template walker (issue #97 stretch goal) is the right
/// solution rather than expanding this list.
///
/// **Baseline note (#97):** the entries below were grandfathered in when
/// the linter shipped — each was confirmed to either (a) have its bit set
/// by a sibling chain in the same mission scope, or (b) inherit the bit
/// from the entity template default (typically `INT_NormalLoot` on
/// lootable bodies, `INT_Trainer` on trainer NPCs, or static dialog
/// NPCs whose `interaction_type` column is non-NULL). The baseline is
/// what made `HackTheRings_Switch` and `Preparation_RingSwitch`
/// detectable — they were NEW additions without a sibling. New chains
/// should default to wiring `set_interaction_type` themselves; only add
/// to this allowlist with a one-line `// reason:` comment explaining
/// where the bit actually comes from.
fn allowlist(filename: &str, chain_id: i32) -> bool {
    matches!(
        (filename, chain_id),
        // castle_cellblock_chains.sql — baseline
        ("castle_cellblock_chains.sql", 1016) // 329_CellDoorButton: bit set by chain 1014/1015 (talk-to-329 dialog choices)
        | ("castle_cellblock_chains.sql", 1032) // ArmYourself_AmbernolVial: INT_NormalLoot from vial template default
        | ("castle_cellblock_chains.sql", 1051) // Preparation_ColMarsh: dialog NPC template default
        | ("castle_cellblock_chains.sql", 1055) // Preparation_SMG1A: lootable body template default
        // TODO(#97): chain 1060 (Preparation_Terminal → Livewire) probably
        // wants `set_interaction_type INT_MinigameLivewire` on the terminal
        // when step 3564 advances — same shape as the chain 1034 fix for
        // HackTheRings_Switch. Verify via UAT or python reference before
        // adding the action; if the terminal template already has the bit,
        // remove this entry and document below.
        | ("castle_cellblock_chains.sql", 1060) // Preparation_Terminal: needs verification (see above TODO)
        // sgc_w1_chains.sql — baseline (dialog NPCs / quest items / lootable bodies)
        | ("sgc_w1_chains.sql", 3002) // SGCW1_GenHammond: dialog NPC template default
        | ("sgc_w1_chains.sql", 3004) // SGC_W1_Tealc: dialog NPC template default
        | ("sgc_w1_chains.sql", 3006) // SGC_W1_ElevatorButton1: bit set by sibling chain
        | ("sgc_w1_chains.sql", 3008) // SGC_W1_FirearmBody: lootable body template default
        | ("sgc_w1_chains.sql", 3012) // SGCW1_GenHammond: dialog NPC template default (second chain)
        | ("sgc_w1_chains.sql", 3020) // SGCW1_AirmanBody: lootable body template default
        | ("sgc_w1_chains.sql", 3023) // SGC_W1_NaqBomb: mission-object template default
        | ("sgc_w1_chains.sql", 3028) // SGC_W1_ElevatorButton2: bit set by sibling chain
        // space_castle_cellblock_chains.sql — baseline
        | ("space_castle_cellblock_chains.sql", 5014) // Preparation_ColMarsh: dialog NPC template default
        | ("space_castle_cellblock_chains.sql", 5015) // Preparation_ColMarsh: dialog NPC template default
    )
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
         allowlist entry in tests/interact_tag_linter.rs::allowlist with \
         a comment explaining why.",
        violations.len(),
        violations.join("\n"),
    );
}

/// Scan a chain SQL file for `enter_region` / `exit_region` trigger
/// event_keys. Returns each (chain_id, region_tag) pair found.
///
/// `region_tag` is the full `'World.Region'` string from the trigger row
/// — we rely on the case-sensitive equality match in
/// `engine.resolve_event` (event_dispatch.rs::fire_enter_region), so a
/// typo like `Castle_CellBlock.Region9` vs the canonical
/// `Castle_Cellblock.Region9` (point_sets row) silently never matches.
fn scan_region_triggers(sql: &str) -> Vec<(i32, String)> {
    let mut out = Vec::new();
    for line in sql.lines() {
        let trimmed = line.trim_start();
        let cursor = trimmed.trim_start_matches("VALUES").trim_start();
        if !cursor.starts_with('(') {
            continue;
        }
        if !cursor.contains("'enter_region'") && !cursor.contains("'exit_region'") {
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

        // Layout: (chain_id, 'enter_region'|'exit_region', 'World.Region', 'scope', ...)
        // Quoted strings: ['enter_region', 'World.Region', 'scope']
        if let Some(region) = quoted.get(1) {
            out.push((chain_id, region.to_string()));
        }
    }
    out
}

/// Region-tag world prefixes are case-sensitive. The content engine
/// resolver does a literal string match on the trigger event_key
/// (event_dispatch.rs::fire_enter_region passes the region tag
/// through unchanged), and the canonical region tag comes from the
/// `point_sets` table. If a chain seeds `Castle_CellBlock.Region9`
/// (capital `B`) but `point_sets` has `Castle_Cellblock.Region9`
/// (lowercase `b`), the trigger never fires and the player is
/// silently soft-stuck on whatever step the chain was meant to
/// advance.
///
/// This is exactly what happened in chain 1073 of mission 680
/// ("Lockdown! Find another way out of the Castle!") — every other
/// region trigger in `castle_cellblock_chains.sql` uses
/// `Castle_Cellblock.*`, only chain 1073 had the typo.
///
/// The invariant: within a single chain SQL file, every world prefix
/// (the part before the first `.`) must use a single, consistent
/// case. Cross-file consistency is also worth checking but is not
/// strictly required (different files describe different worlds).
#[test]
fn every_chain_file_uses_consistent_region_world_prefix() {
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
        let triggers = scan_region_triggers(&sql);

        // world_lower → (canonical_case, first_chain_id_seen) for the first
        // occurrence; any later trigger with a different case on the same
        // lowercase world is a violation.
        let mut seen: HashMap<String, (String, i32)> = HashMap::new();
        for (chain_id, region) in &triggers {
            let world = match region.split_once('.') {
                Some((w, _)) => w,
                None => continue, // No world prefix; skip rather than erroring.
            };
            let key = world.to_ascii_lowercase();
            match seen.get(&key) {
                Some((canonical, first_chain)) if canonical != world => {
                    violations.push(format!(
                        "  {}: chain {} uses world prefix '{}' but chain {} \
                         (earlier in file) uses '{}' for the same world — \
                         the resolver does case-sensitive string matching, \
                         so one of these will never fire",
                        filename, chain_id, world, first_chain, canonical,
                    ));
                }
                Some(_) => {}
                None => {
                    seen.insert(key, (world.to_string(), *chain_id));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Chain seed files have inconsistent region-world casing — the \
         content engine matches event_keys with case-sensitive string \
         equality, so an inconsistent prefix means the trigger never \
         fires and the player is soft-stuck:\n{}",
        violations.join("\n"),
    );
}

#[test]
fn scan_region_triggers_extracts_chain_and_region() {
    let sql = r#"
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1, 'enter_region', 'World_A.Region2', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (2, 'exit_region', 'World_A.Region3', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3, 'interact_tag', 'NotARegion', 'player', false, 0);
"#;
    let triggers = scan_region_triggers(sql);
    assert_eq!(triggers.len(), 2, "must skip non-region triggers");
    assert_eq!(triggers[0], (1, "World_A.Region2".to_string()));
    assert_eq!(triggers[1], (2, "World_A.Region3".to_string()));
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
