//! Per-file `interact_tag` allowlist, split out of `mod.rs` to keep it under the cap.

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
pub(super) fn allowlist(filename: &str, chain_id: i32) -> bool {
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
        | ("castle_cellblock_chains.sql", 11003) // Prisoner_329 (CB-N05): per-player bind, dsm 2795 slot 17 (flags 1073741824) by chains 11000/11001/11002; template 17 interaction_type 0
        | ("castle_cellblock_chains.sql", 11033 | 11034 | 11036 | 11037) // Cellblock_Ambient_Prisoner1/2 (CB-N06): per-player bind, dsm 3862 slot 176 (flags 1073741824) by chains 11030/11031/11032; template 176 interaction_type 0
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
        // Dakara_E1 mission chains (DK-10a to DK-33). reason: every NPC and
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
        | ("dakara_e1_m1648_chains.sql", 8180..=8199)
        | ("dakara_e1_m1649_offer_chains.sql", 8200..=8219)
        | ("dakara_e1_m1649_terminal_chains.sql", 8220..=8249)
        | ("dakara_e1_m1650_chains.sql", 8250..=8289)
        | ("dakara_e1_m1651_chains.sql", 8290..=8309)
        | ("dakara_e1_m1651_gate_chains.sql", 8310..=8319)
        | ("dakara_e1_m1652_chains.sql", 8320..=8349)
        | ("dakara_e1_m1653_chains.sql", 8350..=8389)
        | ("dakara_e1_m1654_chains.sql", 8430..=8449)
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
        | ("harset_opcore_chains.sql", 6501) // CmdCenter_Marsh: dsm 120101 bound by chain 6502
        | ("harset_opcore_chains.sql", 6503) // CmdCenter_Copplemann: dsm 2817 bound by chain 6504
        | ("harset_opcore_chains.sql", 6512) // CmdCenter_Marsh: dsm 5254 bound by chain 6511
        | ("harset_opcore_chains.sql", 6515) // CmdCenter_Mohkatan: dsm 6397 bound by chains 6513/6514
        | ("harset_opcore_chains.sql", 6520) // CmdCenter_Mohkatan: dsm 6398 bound by chain 6519
        | ("harset_opcore_chains.sql", 6522) // CmdCenter_Baal: dsm 6395 bound by chains 6520/6521
        | ("harset_opcore_chains.sql", 6527) // CmdCenter_Marsh: dsm 5253 bound by chains 6525/6526
        // harset_1664_chains.sql: mission 1664 (FZ-20). Cursors come from the template and per-player binds.
        | ("harset_1664_chains.sql", 6008..=6011) // Harset_Kaerlia: template 260's own INT_NonAStoryMissionAvaliable (134217728), as template 390
        | ("harset_1664_chains.sql", 6012..=6013) // Harset_CmdDoor: per-player dsm 120301 bind (INT_MissionWorldObject) by chains 6008/6015
        | ("harset_1664_chains.sql", 6018..=6019) // CmdCenter_Marsh / CmdCenter_Mohkatan: per-player dsm 120302/120303 binds (INT_NonAStoryMissionTurnIn) by chains 6033/6034
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
        // tollana_568_chains.sql — mission 568 (tollana-continuation T-03a1).
        // reason: Copplemann's bit comes from the per-player dialog bind on slot 420.
        | ("tollana_568_chains.sql", 9001..=9003) // Tollana_Copplemann: per-player bind (2656/4959/6393), not a zone-wide flag
        | ("tollana_568_chains.sql", 9011..=9018) // Tollana_FoodUnit_*: template 421 default INT_MissionWorldObject (1073741824), as Dakara templates 445-448
    ) || super::beta_e1::allowed(filename, chain_id)
}
