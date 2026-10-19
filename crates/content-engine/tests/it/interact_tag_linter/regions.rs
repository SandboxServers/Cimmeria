//! Region-key lint: every `enter_region` / `exit_region` trigger key in a
//! chain seed file byte-matches a seeded `point_sets.name`. Moved out of
//! `interact_tag_linter.rs` by DK-09b, which also made the point-set scan
//! read every `point_sets*.sql` seed file instead of `point_sets.sql` alone.

use std::collections::HashSet;
use std::fs;

use super::workspace_root;

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

/// Scan one point-set seed file for `INSERT INTO point_sets (set_id, name, ...)`
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

/// The `name` of every `point_sets` row in every `point_sets*.sql` file of
/// `db/resources/Events/Seed/` (`point_sets.sql` plus the per-zone files,
/// such as `point_sets_dakara_e1.sql`), so a zone's own regions are
/// linted the moment its file lands.
fn seeded_point_set_names() -> HashSet<String> {
    let dir = workspace_root().join("db/resources/Events/Seed");
    let mut names = HashSet::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        let file = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !(file.starts_with("point_sets") && file.ends_with(".sql")) {
            continue;
        }
        let sql =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        names.extend(scan_point_set_names(&sql));
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
/// this check and [`super::every_interact_tag_chain_has_set_interaction_type`]
/// `read_dir` the seed directory and take every `*_chains.sql` file, so
/// a newly added seed file is linted the moment it lands. Registering it
/// for the *loader* is a separate step, and neither of those two checks
/// notices when it is missed — that is what
/// [`super::every_chain_seed_file_is_registered_in_database_sql`] is for.
///
/// **Point-set coverage.** The names come from every `point_sets*.sql` seed
/// file (DK-09b). The check used to read `point_sets.sql` alone, so a region
/// seeded in a per-zone file, such as Dakara_E1's 2123-2130, failed it.
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
    let canonical_names = seeded_point_set_names();
    assert!(
        !canonical_names.is_empty(),
        "point_sets*.sql scan found zero names — parser drift, not an \
         empty seed dir"
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

/// DK-09b revert proof: the scan reads `point_sets.sql` AND the per-zone
/// point-set files. Going back to `point_sets.sql` alone loses the Dakara
/// name and fails this test.
#[test]
fn point_set_scan_reads_every_point_sets_seed_file() {
    let names = seeded_point_set_names();
    for name in ["Castle_CellBlock.Region8", "Dakara_E1.CommandTent"] {
        assert!(
            names.contains(name),
            "{name} is missing: the scan must read point_sets.sql and every point_sets_*.sql"
        );
    }
}
