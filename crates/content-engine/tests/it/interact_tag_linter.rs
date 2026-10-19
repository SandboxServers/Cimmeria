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

// The region-key lint (DK-09b split it out to keep this file under the cap).
mod regions;

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
        | ("castle_cellblock_chains.sql", 5030) // BlackMarket_Auctioneer: template 305 default INT_Auction (BM-07); the bit is also the auctioneer marker, so it must stay seeded
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
        // family, the largest single group here. The second largest, and the
        // one that grows with every Harset mission packet, is per-player
        // dialog binds (castle_701, harset_jaffa, harset_opcore). Counts are
        // deliberately not written down: two lanes merged with stale tallies
        // on the same day. `grep -c '| ("' ` this file for the total.
        | ("harset_space_chains.sql", 6001) // HarsetRingLeftBottom
        | ("harset_space_chains.sql", 6002) // HarsetRingRightBottom
        | ("harset_space_chains.sql", 6003) // HarsetRingLeft
        | ("harset_space_chains.sql", 6004) // HarsetRingLeftTop
        | ("harset_space_chains.sql", 6005) // HarsetRingRight
        // dakara_e1_space_chains.sql — DK-04's four flap props use templates
        // 445-448. DK-03 seeded each template with INT_MissionWorldObject
        // (1073741824), and its live-DB guard checks the bit after spawn.
        // The flap is clickable before its travel chain fires, so adding
        // set_interaction_type to the triggered chain would be too late.
        | ("dakara_e1_space_chains.sql", 8003) // ToCommand: template 445
        | ("dakara_e1_space_chains.sql", 8004) // ToMohkatan: template 446
        | ("dakara_e1_space_chains.sql", 8005) // FromCommand: template 447
        | ("dakara_e1_space_chains.sql", 8006) // FromMohkatan: template 448
        // dakara_e1_m*_chains.sql and dakara_e1_bridge_chains.sql — the
        // Dakara_E1 mission chains (DK-10a to DK-12c). reason: every NPC and
        // prop these chains click gets its interaction bit from the PER-PLAYER
        // dialog bind (`add_dialog_set` pushes the dialog_set_map row's
        // `interaction_flags` to the binding player only), as in castle_701
        // (D-CA15). A `set_interaction_type` would light the prop for every
        // player in world 61 at once.
        | ("dakara_e1_m1570_entry_chains.sql", 8040..=8044)
        | ("dakara_e1_m1570_tent_chains.sql", 8045..=8059)
        | ("dakara_e1_m1570_search_chains.sql", 8060..=8074)
        | ("dakara_e1_bridge_chains.sql", 8075..=8079)
        | ("dakara_e1_m1645_chains.sql", 8080..=8119)
        | ("dakara_e1_m1646_offer_chains.sql", 8120..=8139)
        | ("dakara_e1_m1646_drop_chains.sql", 8140..=8159)
        | ("dakara_e1_m1570_report_chains.sql", 8160..=8179)
        // castle_701_chains.sql — mission 701 (Gerschon / Copplemann).
        // reason: both NPCs' interaction bit comes from the PER-PLAYER
        // dialog bind, not from a zone-wide flag. `add_dialog_set` pushes
        // the dialog_set_map row's `interaction_flags` to the binding
        // player only (executor/dialog.rs::send_interaction_update_if_visible),
        // which is what decision D-CA15 requires: a `set_interaction_type`
        // here would light the "!" over Gerschon and Copplemann for every
        // player in the shared world at once, including players who have
        // already finished 701. Templates 149 and 48 both carry
        // `interaction_type = 0` and `static_interaction_sets = '{}'`, so
        // the bind is the only source of the bit — see the GAP 1 block in
        // castle_701_chains.sql for why that bind is a no-op until packet
        // CA02 widens `DialogSetMapEntry.dialog_id` to `Option<i32>`.
        | ("castle_701_chains.sql", 1202) // Castle_SgtGerschon: per-player bind, dsm 3062 (chain 1201)
        | ("castle_701_chains.sql", 1203) // Castle_SgtGerschon: per-player bind, dsm 3062 (chain 1201)
        | ("castle_701_chains.sql", 1231) // Castle_Coppleman: per-player bind, dsm 3062 (chains 1204/1205/1240)
        | ("castle_701_chains.sql", 1233) // Castle_Coppleman: per-player bind, dsm 3062 (chains 1204/1205/1241)
        | ("castle_701_chains.sql", 1236) // Castle_Coppleman: per-player bind, dsm 3063 (chains 1235/1243)
        // harset_jaffa_chains.sql — Harset Loyalist Jaffa talk chains
        // (packets H20/H22). reason: these are the FIRST entries in this
        // allowlist whose bit comes from neither a sibling chain nor an
        // entity-template default — it comes from a per-player
        // `add_dialog_set` bind, and that is a correctness requirement
        // rather than a style choice.
        //
        // Templates 42 (Ba'al), 54 (Moh'katan) and 204 (Ra's Former
        // Jaffa) all carry `entity_templates.interaction_type = 0`, so
        // something must supply the bit. `set_interaction_type` cannot:
        // it mutates `CellEntity::interaction_type_flags` and fans the
        // new value to every witness, and worlds 57/68 are SHARED
        // persistent hubs (Harset_CmdCenter is `Instanced="false"`), so
        // one player clearing an icon would un-click the NPC for every
        // other player still on the step. The Harset campaign's
        // architecture guardrail (docs/analysis/harset-rebuild/README.md,
        // "Architecture Guardrails") forbids it outright on shared-hub
        // NPCs. `add_dialog_set` writes the firing player's
        // `available_interactions` and pushes
        // `base_flags | entry.interaction_flags` to that player alone.
        //
        // Each chain below has a matching `add_dialog_set` for its NPC's
        // template in the same file — 120001 on slot 54, 5151 on slot 42,
        // 120002 on slot 204 — plus a `player_loaded` restore chain,
        // since binds are in-memory and die on a cross-world hop. The
        // linter cannot see that pairing because it matches on
        // `set_interaction_type`'s *tag* and `add_dialog_set` addresses a
        // *template id*; teaching it the tag→template join needs the
        // entity-template walker of #97.
        | ("harset_jaffa_chains.sql", 6304) // CmdCenter_Baal: dsm 5151 bound by chains 6303/6306
        | ("harset_jaffa_chains.sql", 6305) // CmdCenter_Mohkatan: dsm 120001 bound by chains 6304/6307
        | ("harset_jaffa_chains.sql", 6335) // Harset_FormerRaJaffa: dsm 120002 bound by chain 6334
        | ("harset_jaffa_chains.sql", 6336) // Harset_FormerRaJaffa2: same bind, same template (204)
        | ("harset_jaffa_chains.sql", 6340) // CmdCenter_Mohkatan: dsm 5161 bound by chain 6339
        // harset_opcore_chains.sql — Harset OP-CORE hub NPCs (packets H30, H31).
        // reason: the INT bit for every one of these comes from a PER-PLAYER
        // `add_dialog_set` bind, not from a template default and not from a
        // `set_interaction_type` action. Worlds 57 and 68 are SHARED spaces
        // (entities/spaces.xml `Instanced="false"`, decision D-H04), and
        // `set_interaction_type` mutates the NPC's own
        // `interaction_type_flags` and broadcasts to every witness — using it
        // for a mission indicator would light Col. Marsh up for every player
        // in the Command Center the moment one player qualified.
        // `add_dialog_set` instead writes the acting player's
        // `available_interactions` and pushes InteractionType to that witness
        // alone (executor/dialog.rs::send_interaction_update_if_visible,
        // plus the AoI-create path in space_manager/aoi.rs for NPCs not yet
        // witnessed), which is the only per-player mechanism the engine has.
        // The bit is real; it just arrives per-player. Each chain's binding
        // sibling is named below.
        //
        // NOTE (#97): this is the third multi-entry batch in this allowlist,
        // and the "bit comes from elsewhere" family is now well over half of
        // it. Every remaining Harset mission packet (H32, H33, H36,
        // H37, H41...) will add more of exactly this shape. The right fix is
        // a per-file exemption rule for dialog-set-driven files, or the #97
        // entity-template walker — NOT another nine entries per packet.
        // Filed as a follow-up in docs/analysis/harset-rebuild/worknotes/H30-H31.md.
        | ("harset_opcore_chains.sql", 6501) // CmdCenter_Marsh: dsm 5356 bound by chain 6502
        | ("harset_opcore_chains.sql", 6503) // CmdCenter_Copplemann: dsm 2817 bound by chain 6504
        | ("harset_opcore_chains.sql", 6512) // CmdCenter_Marsh: dsm 5254 bound by chain 6511
        | ("harset_opcore_chains.sql", 6515) // CmdCenter_Mohkatan: dsm 6397 bound by chains 6513/6514
        | ("harset_opcore_chains.sql", 6520) // CmdCenter_Mohkatan: dsm 6398 bound by chain 6519
        | ("harset_opcore_chains.sql", 6522) // CmdCenter_Baal: dsm 6395 bound by chains 6520/6521
        | ("harset_opcore_chains.sql", 6527) // CmdCenter_Marsh: dsm 5253 bound by chains 6525/6526
        // sgc_w1_chains.sql — baseline (dialog NPCs / quest items / lootable bodies)
        | ("sgc_w1_chains.sql", 3002) // SGCW1_GenHammond: dialog NPC template default
        | ("sgc_w1_chains.sql", 3004) // SGC_W1_Tealc: dialog NPC template default
        | ("sgc_w1_chains.sql", 3006) // SGC_W1_ElevatorButton1: bit set by sibling chain
        | ("sgc_w1_chains.sql", 3008) // SGC_W1_FirearmBody: lootable body template default
        | ("sgc_w1_chains.sql", 3012) // SGCW1_GenHammond: dialog NPC template default (second chain)
        | ("sgc_w1_chains.sql", 3020) // SGCW1_AirmanBody: lootable body template default
        | ("sgc_w1_chains.sql", 3023) // SGC_W1_NaqBomb: mission-object template default
        | ("sgc_w1_chains.sql", 3028) // SGC_W1_ElevatorButton2: bit set by sibling chain
        // debug_hub_chains.sql — both NPCs are reusable and belong to no
        // mission, so the cursor bit is a permanent template default rather
        // than a set/clear pair.
        | ("debug_hub_chains.sql", 7001) // DebugHub_DialogNpc: template 302 default INT_NonAStoryMissionAvaliable
        | ("debug_hub_chains.sql", 7004) // DebugHub_LivewireTerminal: template 303 default INT_MinigameLivewire
        | ("debug_hub_chains.sql", 7020) // DebugHub_LootCrate: template 304 default INT_NormalLoot
        | ("castle_702_704_chains.sql", 1274) // Castle_PreRomneyChest: template 410 default INT_NormalLoot
        | ("castle_702_704_chains.sql", 1275) // Castle_PreRomneyChest: template 410 default INT_NormalLoot
        | ("castle_702_704_chains.sql", 1276) // Castle_PreRomneyChest: template 410 default INT_NormalLoot
        | ("debug_hub_chains.sql", 7010) // DebugHub_MailClerk: template 390 default INT_NonAStoryMissionAvaliable (SS-U3)
        // debug_area_plaza_chains.sql (DA-02): the Debug Area plaza's
        // reusable NPCs, no mission, so every cursor bit is a permanent
        // template default, as in the stasis hub.
        | ("debug_area_plaza_chains.sql", 13000) // DebugArea_AbilityGranter: template 1300 default INT_Trainer
        | ("debug_area_lineup_chains.sql", 14090..=14095) // DebugArea_LineupAttendant_*: templates 1590-1595 default INT_Trainer
        | ("debug_area_plaza_chains.sql", 13001) // DebugArea_AbilityReset: template 1301 default INT_Trainer
        | ("debug_area_plaza_chains.sql", 13002) // DebugArea_DialogNpc: template 302 default INT_NonAStoryMissionAvaliable
        | ("debug_area_plaza_chains.sql", 13004) // DebugArea_LivewireTerminal: template 303 default INT_MinigameLivewire
        | ("debug_area_plaza_chains.sql", 13006) // DebugArea_LootCrate: template 304 default INT_NormalLoot
        | ("debug_area_plaza_chains.sql", 13007) // DebugArea_MailClerk: template 390 default INT_NonAStoryMissionAvaliable
        | ("debug_area_plaza_chains.sql", 13008) // DebugArea_Auctioneer: template 305 default INT_Auction, the seeded auctioneer marker
        // debug_area_ring_chains.sql — the nine Debug Area ring consoles
        // (DA-08, DA-11). Template 3 carries INT_RingNetwork (32) as a template
        // default, exactly as for the Harset ring switches above.
        | ("debug_area_ring_chains.sql", 13810..=13818)
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
