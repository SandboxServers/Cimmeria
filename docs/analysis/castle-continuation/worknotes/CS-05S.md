# CS-05S worknote: return-hub services, repeatables and barks seal

> Type: worknote. Packet CS-05S of the Castle continuation R3 packet set, section "CS-05S". Branch `castle/cs-05s-hub-seal`, worktree `castle-cs-05s`.
> Written 2026-10-18 by the CS-05S coder from repo seed files only; corrected by the R3a review the same day. No live server, DB `sgw_castle_cs_01b` or `nav_inspect` was run.
> Verdict: no leg is `SEALED`. Services and barks are blocked; the repeatables leg has a usable count-cap primitive. The economy and authority legs were not run (see Open items).

## Verdict

| Leg | Verdict | Reason |
|---|---|---|
| Services (medic and/or quartermaster) | `BlockedEvidence (services: no vendor-capable Castle template; spot, nav proof, stock and economy review not run)` | No world-8 spawn uses a template with a vendor list (see Services). A new template and spawn needs a `nav_inspect` spot and the `## Economy` stock review. |
| Repeatables (1-2) | `BlockedDecision (D-CA16: no stock repeatable Castle mission, so repeatable_q1 is authored)` | The count cap exists (`num_repeats`, `repeats`); no time cooldown was found; no Castle stock mission is repeatable (see Repeatables). The authority leg did not run, so the cap is not sealed. |
| Barks (2-4) | `BlockedDecision (hub bark copy: which existing Castle dialog_screens rows may be reused, or are new screens authored from the 10800-10919 block)` | The `npc_bark` pattern exists, but no Castle screen row was identified for a hub bark (see Barks). |

## Services

- Castle Medic: `entity_templates` row 177 (`db/resources/Entities/Seed/entity_templates.sql:553`), `template_name` `Castle_Medic`, `name_id` 6964 (`DN_Castle_Medic_Pvt_1-3`, text "Castle Medic", `texts.sql:52976`), `class` `mob`, `interaction_type` 0. One world-8 spawn uses it.
- Its `buy_item_list`, `sell_item_list`, `repair_item_list` and `recharge_item_list` are all NULL, and `trainer_ability_list_id` is NULL. The seeded medic therefore offers no vendor, heal or repair service.
- Review check: the 105 world-8 rows of `db/resources/Worlds/Seed/spawnlist*.sql` reference 35 distinct templates. For every one of them, `buy_item_list`, `sell_item_list`, `repair_item_list` and `recharge_item_list` are all NULL. No existing Castle actor can be a vendor without new rows.
- Vendor mechanics exist in the engine (`buy_item_list` and `sell_item_list` columns; vendor purchase tests in `crates/base-methods/src/base/world_entry/methods/vendor/`). A hub service would need a new template whose lists are populated, and a spawn. Both need `nav_inspect` exit 0 and the aggro clearance, which this worknote did not collect.
- Quartermaster: no Castle quartermaster template exists in the seeds (see the check above).

## Spawn spot

Not chosen. The spot must be on the component connected to the Castle arrival and the Infirmary, at least 5 u outside every aggro radius and outside every hostile respawn radius. None of these was checked, although the spec allows `nav_inspect` in this wave. Template and spawn ID picks are deferred until the spot is measured.

## Stock (`## Economy`)

Not run. The items-systems-advisor leg has not been executed. No stock rows were proposed, so no buy-versus-sell check applies yet.

## Repeatables

- `resources.missions.num_repeats` is read into `cell-catalog` (`crates/cell-catalog/src/cell/spawner/missions.rs:25-28,56`).
- The offer guard refuses a re-accept when `repeats > num_repeats` (`crates/cell-content/src/cell/missions/lifecycle.rs:95`; `crates/entity/src/missions.rs:52-57`). `repeats` is bumped by `complete()` and by `fail()` (`entity/src/missions.rs:102,112`). So a mission with `num_repeats = N` can be completed N + 1 times in total.
- The persisted `repeats` counter is advanced by an UPSERT. `crates/services/src/mission_round_trip_tests.rs:219` (`live_db_second_completion_advances_persisted_repeats_counter_end_to_end`) covers the second completion.
- A count cap is therefore available. No time cooldown was found for missions (a search of `cell-content`, `cell-catalog`, `entity` and the Missions seeds found only ability and mail cooldowns), so no CS-05E primitive is needed for a count-capped repeatable.
- Stock repeatable Castle mission: **none**. Review check of `db/resources/Missions/Seed/missions.sql`: the Castle missions 701, 702, 703, 704, 706, 708, 1360, 1376, 1520, 1521, 1664 and 567 all have `num_repeats` 0 or 1. Across the whole seed, 1029 missions have 1, 11 have 0, and the only mission with more is 971 (`Search and Rescue`, 4), which is not a Castle contract row (`evidence/castle-contract.csv`). So there is no stock mission to prefer, and `repeatable_q1` would be an authored mission under D-CA16.
- Proposed cap: not sealed. "2 completions per player" is `num_repeats = 1`, the seed default; a distinct cap needs an authored `num_repeats`. The authority leg must confirm it is per player and server-side (the counter is per character, saved with the mission row).

## Barks

- Pattern: `npc_bark` action, as in `db/resources/Content/Seed/castle_cellblock_chains.sql` chains 1176-1178 (chain 1176 at line 3159, 1178 at line 3258). Each bark is a `content_actions` row with `screen_id`, `speaker` and `channel`.
- Correction to the coder draft: those chains are the **Cellblock** Marsh escort barks (DU-07 of GC1, scoped to missions 680 and 681; the file header at lines 43-52 says so). The file name says "castle" but they are not Castle content. They are the pattern precedent only. The screens they use are `dialog_screens` rows of dialog 5019: screen 96351 "Let's move out!", 96352 (Mess Hall flank cue), 96354 (Hallway05 flank cue); screen 96353 is deliberately unauthored (chain 1178 comment).
- No Castle `npc_bark` chain exists, and no Castle `dialog_screens` row was identified as a hub bark. This review did not search the Castle dialogs for reusable lines.
- Chain IDs: the coder draft said 1179-1190 are free. That is **false**: chains 1181-1195 are defined in the same file (1181-1190 at lines 1908-2112, and 1191-1195; the header at lines 56-60 records 1192-1195), so 1179-1180 are the only free IDs in that range. Hub bark chains do not come from there anyway. The CS-05 chain IDs come from the Castle authored chain sub-block 10000-10299 proposed in `evidence/mission-catalog.md` section 2 (new bark screens would come from 10800-10919). No ID was picked, and none is invented here.
- Hub barks (2-4) need either an existing Castle screen row or new `dialog_screens` rows. Either is a decision, so the verdict is a `BlockedDecision`.

## Authority

Not run. The server-authority-enforcer leg has not been executed. Reward limits for repeatables and vendor sell paths are unreviewed.

## Frozen rows

None. No leg is `SEALED`, so no SQL or Rust is frozen and no consuming packet may start.

## Test lists (proposed, not run)

- CS-05a `live_db_castle_hub` (consumes services): spawn row, template service type, stock rows, aggro clearance. Blocked on the services verdict.
- CS-05b `castle_repeatable` replay (consumes repeatables): offer, complete, re-offer, refusal at the cap. The count-cap primitive is available; the packet is blocked on D-CA16.
- CS-05c `castle_hub_barks` replay (consumes barks): each bark fires its one screen once per player. Blocked on the bark decision above.

## End-UAT rows

None yet. Nothing is sealed, so there is nothing for the user to run. Rows are added when a leg is sealed.

## Open items

1. Run `nav_inspect` for a hub spot on the component connected to the Castle arrival and the Infirmary, and the aggro and respawn clearance (the lane allows it for this wave).
2. Choose services: a new template with vendor lists plus a spawn (no existing Castle template qualifies).
3. Run the items-systems-advisor (`## Economy`) and server-authority-enforcer (`## Authority`) legs.
4. Decide hub barks: search the Castle dialogs for reusable screens, or author new ones from the 10800-10919 block.
5. Answer D-CA16 for `repeatable_q1`, since there is no stock repeatable Castle mission.
