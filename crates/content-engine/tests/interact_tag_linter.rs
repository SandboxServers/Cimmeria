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
        // harset_space_chains.sql — the five Harset ring switches (packet H10).
        // reason: all five spawns (spawnlist.sql rows 4/127/128/129/130) use
        // entity template 3 "Ring Transporter Switch", whose
        // `entity_templates.interaction_type` column is already 32
        // (INT_RingNetwork). The spawner reads that column onto every spawned
        // entity, so the bit is present from spawn and survives restart —
        // a `set_interaction_type` action would be redundant and would imply
        // the bit needs setting. Same template as the three already-shipped
        // Cellblock ring switches (spawns 17/23/79). See the seed file's
        // "RING SWITCHES" header comment.
        //
        // NOTE (#97): five entries added at once is right at the threshold this
        // allowlist's own doc comment calls out — the entity-template walker is
        // the real fix for the whole "bit comes from the template default"
        // family, which is now 16 of the 20 entries here.
        | ("harset_space_chains.sql", 6001) // HarsetRingLeftBottom
        | ("harset_space_chains.sql", 6002) // HarsetRingRightBottom
        | ("harset_space_chains.sql", 6003) // HarsetRingLeft
        | ("harset_space_chains.sql", 6004) // HarsetRingLeftTop
        | ("harset_space_chains.sql", 6005) // HarsetRingRight
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

/// Scan `point_sets.sql` for `INSERT INTO point_sets (set_id, name, ...)`
/// rows and return the set of canonical `name` values (byte-exact,
/// including case). `name` is the first quoted string in each VALUES
/// tuple (the `set_id` column ahead of it is numeric, not quoted).
fn scan_point_set_names(sql: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    for line in sql.lines() {
        // Each `INSERT INTO point_sets (...) VALUES (...)` is one
        // physical line in this file (unlike the chain seed files,
        // point_sets.sql never wraps a single INSERT across multiple
        // lines), so a substring check on the line is sufficient.
        if !line.contains("INSERT INTO point_sets") {
            continue;
        }
        let quoted: Vec<&str> = line
            .split('\'')
            .enumerate()
            .filter_map(|(i, s)| if i % 2 == 1 { Some(s) } else { None })
            .collect();
        if let Some(name) = quoted.first() {
            names.insert(name.to_string());
        }
    }
    names
}

/// Region-tag event_keys must byte-exactly match a real `point_sets.name`
/// row. The content engine resolver does a literal, case-sensitive
/// string match on the trigger event_key
/// (`event_dispatch.rs::fire_enter_region` passes the region tag
/// through unchanged), so any chain whose `enter_region`/`exit_region`
/// trigger key isn't a real point-set name — wrong case, typo, or a
/// region that was never seeded — silently never fires, soft-stucking
/// the player on whatever step the chain was meant to advance.
///
/// This used to be an internal per-file "one canonical case per world
/// prefix" heuristic (it caught chain 1073 of mission 680, "Lockdown!
/// Find another way out of the Castle!", which typo'd
/// `Castle_CellBlock.Region9` against the file's own
/// `Castle_Cellblock.*` convention). That heuristic produces a false
/// positive for Castle_CellBlock's Region8: `point_sets.sql` itself
/// spells every other Castle_Cellblock region with a lowercase `b`
/// EXCEPT Region8, which is `Castle_CellBlock.Region8` (capital `B`,
/// set_id 2039) — a genuine, singular inconsistency in the shipped
/// game data (see audit.md defect B3 in
/// `docs/analysis/castle-cellblock-rebuild/`). Checking against the
/// real `point_sets` table instead of file-internal consistency is
/// both stricter (catches a lone bad key with no correct sibling to
/// compare against, which the old heuristic could miss) and correctly
/// allows Region8's legitimate exception.
///
/// **Scan coverage.** There is no per-file scan list to maintain: both
/// this check and [`every_interact_tag_chain_has_set_interaction_type`]
/// `read_dir` the seed directory and take every `*_chains.sql` file, so
/// a newly added seed file is linted the moment it lands. Registering it
/// for the *loader* is a separate step, and neither of those two checks
/// notices when it is missed — that is what
/// [`every_chain_seed_file_is_registered_in_database_sql`] is for.
///
/// **Dotless keys are in scope.** The old per-file heuristic keyed on a
/// `World.Region` prefix and so silently skipped any point-set name
/// without a dot. This check has no such heuristic — it is a plain
/// set-membership test against every `point_sets.name`, dotted or not.
/// `dotless_point_set_names_are_not_skipped` pins that by calling
/// [`region_key_violations`] — the same function this test uses — because
/// the Harset ring point sets (`HarsetRingLeftBottomPS` and friends, set
/// ids 2052-2056) are dotless and a reintroduced prefix heuristic would
/// exempt them from validation without failing anything.
#[test]
fn every_chain_region_key_matches_a_seeded_point_set() {
    let point_sets_path = workspace_root().join("db/resources/Events/Seed/point_sets.sql");
    let point_sets_sql = fs::read_to_string(&point_sets_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", point_sets_path.display()));
    let canonical_names = scan_point_set_names(&point_sets_sql);
    assert!(
        !canonical_names.is_empty(),
        "point_sets.sql scan found zero names — parser drift, not an \
         empty seed file"
    );

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
        violations.extend(region_key_violations(filename, &sql, &canonical_names));
    }

    assert!(
        violations.is_empty(),
        "Chain seed files reference region keys that don't byte-match any \
         seeded point_sets.name — the content engine matches event_keys \
         with case-sensitive string equality, so a mismatched key never \
         fires and the player is soft-stuck:\n{}",
        violations.join("\n"),
    );
}

/// The membership check itself, factored out of
/// [`every_chain_region_key_matches_a_seeded_point_set`] so tests can
/// exercise the *production* predicate rather than re-implementing it.
///
/// That distinction is the whole point. A test that inlines
/// `canonical_names.contains(region)` in its own body asserts that
/// `HashSet::contains` is case-sensitive — which is a property of std,
/// not of this linter — and keeps passing when someone adds a skip
/// heuristic here. Calling this function means a heuristic added here
/// breaks those tests, which is what makes them guards.
fn region_key_violations(
    filename: &str,
    sql: &str,
    canonical_names: &HashSet<String>,
) -> Vec<String> {
    let mut violations = Vec::new();
    for (chain_id, region) in &scan_region_triggers(sql) {
        if !canonical_names.contains(region) {
            violations.push(format!(
                "  {filename}: chain {chain_id} triggers on region_key \
                 '{region}', which is not a byte-exact point_sets.name \
                 — the resolver does case-sensitive string matching, so \
                 this trigger never fires",
            ));
        }
    }
    violations
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

#[test]
fn scan_point_set_names_extracts_the_name_column_byte_exact() {
    let sql = r#"
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2039, 'Castle_CellBlock.Region8', 'AreaSet', 12, NULL, 0, 'BoundingBox', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2033, 'Castle_Cellblock.Region2', 'AreaSet', 12, NULL, 0, 'BoundingBox', 1);
"#;
    let names = scan_point_set_names(sql);
    assert_eq!(names.len(), 2, "must extract exactly the two seeded names");
    // The two rows differ only in the casing of "Cellblock" — pinning
    // both as distinct set members is the whole point: a HashSet<String>
    // lookup is case-sensitive, so 'Castle_CellBlock.Region8' and
    // 'Castle_Cellblock.Region8' would collide if the scanner
    // normalized case anywhere.
    assert!(names.contains("Castle_CellBlock.Region8"));
    assert!(names.contains("Castle_Cellblock.Region2"));
    assert!(!names.contains("Castle_Cellblock.Region8"));
}

/// Regression guard for the false-positive this check used to produce
/// (see the doc comment on `every_chain_region_key_matches_a_seeded_point_set`):
/// a chain file with two DIFFERENT regions that legitimately carry
/// different casing (because that's what point_sets.sql actually has)
/// must not be flagged, while a chain that references a region key
/// with NO matching point_sets row at all — wrong case or a typo — must
/// still be caught. This exercises the check's core matching logic
/// directly against synthetic data, independent of the live seed.
#[test]
fn region_key_check_allows_legitimate_per_region_casing_but_catches_a_mismatch() {
    let point_sets_sql = r#"
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2039, 'Castle_CellBlock.Region8', 'AreaSet', 12, NULL, 0, 'BoundingBox', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2033, 'Castle_Cellblock.Region2', 'AreaSet', 12, NULL, 0, 'BoundingBox', 1);
"#;
    let canonical_names = scan_point_set_names(point_sets_sql);

    let good_chain_sql = r#"
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1008, 'enter_region', 'Castle_CellBlock.Region8', 'player', true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1011, 'enter_region', 'Castle_Cellblock.Region2', 'player', false, 0);
"#;
    let good_triggers = scan_region_triggers(good_chain_sql);
    assert!(
        good_triggers
            .iter()
            .all(|(_, region)| canonical_names.contains(region)),
        "two legitimately differently-cased region keys for DIFFERENT \
         regions must both validate — this is exactly the Region8 shape \
         the old per-file-consistency heuristic false-flagged"
    );

    let bad_chain_sql = r#"
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5002, 'enter_region', 'Castle_Cellblock.Region8', 'player', true, 0);
"#;
    let bad_triggers = scan_region_triggers(bad_chain_sql);
    assert!(
        bad_triggers
            .iter()
            .any(|(_, region)| !canonical_names.contains(region)),
        "a region key with the wrong case for Region8 (lowercase 'b', the \
         auto-export's actual bug — audit.md defect B3) must be flagged \
         even though nothing else in the same synthetic file has the \
         correct casing to compare against; that's the exact case the old \
         per-file-consistency heuristic could miss"
    );
}

/// A point-set name with no `World.Region` dot must still be collected
/// by the scanner and still validate as a region key.
///
/// The Harset ring pads (`point_sets.sql` set ids 2052-2056:
/// `HarsetRingLeftBottomPS`, `HarsetRingRightBottomPS`,
/// `HarsetRingLeftPS`, `HarsetRingLeftTopPS`, `HarsetRingRightPS`) are
/// the live instance of this shape — five real, world-57 `AreaSet` rows
/// whose names carry no world prefix. The superseded per-file
/// "one canonical case per world prefix" heuristic derived the prefix by
/// splitting on `.`, so a dotless key had no prefix bucket and was
/// skipped outright: a chain could reference `HarsetRingLeftBottomPs`
/// (wrong case) and the linter would say nothing.
///
/// The current set-membership check has no prefix logic at all, so it
/// covers them already. This test exists so that stays true — anyone
/// reintroducing prefix-based grouping has to make this pass, and the
/// only way to do that is to keep dotless names inside the checked set
/// rather than exempting them.
///
/// It drives [`region_key_violations`], the same function the production
/// check calls, rather than re-implementing the membership filter in its
/// own body. An earlier version of this test did re-implement it and was
/// a tautology: a skip heuristic added to the production check left the
/// test green, because the test never ran the heuristic.
#[test]
fn dotless_point_set_names_are_not_skipped() {
    let point_sets_sql = r#"
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2052, 'HarsetRingLeftBottomPS', 'AreaSet', 57, 2.52999997, 1.76999998, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2078, 'Harset.CommandCenterTransition', 'AreaSet', 57, NULL, 0, 'BoundingBox', 1);
"#;
    let canonical_names = scan_point_set_names(point_sets_sql);
    assert!(
        canonical_names.contains("HarsetRingLeftBottomPS"),
        "the scanner must collect dotless point-set names — the Harset \
         ring pads (set ids 2052-2056) have no world prefix"
    );

    // 6001 spells the dotless key correctly; 6002 miscases it. Only 6002
    // may be reported.
    let chain_sql = r#"
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6001, 'enter_region', 'HarsetRingLeftBottomPS', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6002, 'enter_region', 'HarsetRingLeftBottomPs', 'player', false, 0);
"#;
    let violations = region_key_violations("synthetic_chains.sql", chain_sql, &canonical_names);
    assert_eq!(
        violations.len(),
        1,
        "exactly one of the two dotless keys is wrong, so the production \
         check must report exactly one violation. Zero means dotless names \
         are being skipped entirely and the Harset ring point sets are \
         unguarded; two means the correctly spelled key was rejected. \
         Got: {violations:?}"
    );
    assert!(
        violations[0].contains("chain 6002"),
        "the reported violation must be the miscased key (chain 6002), not \
         the correct one. Got: {violations:?}"
    );
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
