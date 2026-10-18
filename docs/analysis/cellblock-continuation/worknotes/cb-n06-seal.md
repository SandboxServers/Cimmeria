# CB-N06s seal: Before the Patrol Returns (mission 62060)

> **Type:** Worknote (seal packet CB-N06s)
> **Audience:** The W5 architect, the CB-N06-1..7 coders and CB-N06m
> **Last updated:** 2026-10-17
> **Base:** `278a4e746` (branch `cellblock/cb-n06s-choice-seal`). Read-only: no lab, no build.
> **Spec:** [w5-packets.md](../w5-packets.md) (CB-N06s), [w5-frozen.md](../w5-frozen.md). Format precedent: [cb-n05-seal.md](cb-n05-seal.md).

## Verdict

**seal.** Every claim in w5-frozen.md F1-F5 checks out against code and seeds at the base; no literal needs an edit. G8 (walk-back after the ring-out) and the player-invisible full-bag refusal in G11 are risks for CB-U12 and CB-N06m, not defects.

## Checks

Paths are relative to the repo root.

| Check | Result | Evidence |
|---|---|---|
| G1 Click path | Pass | Spawns 520/521 (template 176) and 22 (template 22) are plain props with no static sets (`spawnlist.sql` 66, 198, 199). An interaction-only bind (`dialog_id` NULL, flags 1073741824) is skipped by `handle_interact` and falls through (`cell-interactions/.../dispatch/interact.rs`); `interact.rs` (cell-methods `player/interaction/`) runs the range gate (5.0 m) and the target pin, then `fire_interact_tag` with the entity tag, the path Prisoner 329's 2795 bind already uses for 11003. The client receives only the InteractionType flags (`aoi.rs` 214-257 dynamicUpdate push via `send_interaction_update_if_visible`, `executor/dialog/mod.rs` 457); the dialog-set topic text is not on the wire, so `Mend Slave Cages` and `Remove Safeguards ...` are never shown. First-press feedback: accept gives the journal entry plus dialog 60151 (nested `mission_accepted`); a branch gives the item grant, the mission-complete frames and the unbind. |
| G2 Template side effects | Pass | Templates 176 (`event_set_id` 570) and 22 carry no item, combat or mission action on click (`entity_templates.sql` 552, 55); the click only reaches the tag dispatch. |
| G3 Panel after 638 | Pass | 1016 requires `step_status 638 '2115' eq active` (`castle_cellblock_chains.sql` 706); 1017 has no trigger row (minigame callback only, 717); 1028 sets the bit on `player_loaded` only while 2115 is active (823-833). With 638 completed none fire, so the panel click starts no Livewire and changes no bit. |
| G4 Single snapshot | Pass | `event_dispatch/interaction.rs` line 55 resolves every chain against the pre-click context and line 68 executes. 62060 reads `not_active` at resolve time, so 11033/11034 cannot be joined by 11036-11038 on the accept click. `AcceptMission` persists, then `fire_mission_accepted` (`event_dispatch/mission.rs` 40-92) rebuilds the context after the accept (step 62061 active) and resolves 11035, as 11004 does for 62050. The same function populates `world_id`, so the `world 12` gate holds. |
| G5 Completion | Pass | `executor/mission.rs` `complete_objective` (366) calls `cell/missions/progression.rs` `complete_objective` (325); 62062 is the only objective of the only step, so the mission auto-completes and `fire_mission_completed` runs. The 11036 order (grant, two unbinds, then the objective) leaves the completion last. |
| G6 World change | Pass | `available_interactions` is initialised empty on every cell-entity construction (`entity/src/cell_entity/construction.rs` 67), rebuilt on a world change and on relog. Template 176 in world 12 is only spawns 520/521 and template 22 only spawn 22 (grep of `spawnlist.sql`), and the restore chains 11031/11032 are gated on world 12 and `Castle_CellBlock`, so no bind reaches world 8 (spawns 193, 199-203). |
| G7 Restores | Pass | 11031 re-adds 3862 when 638 completed, 641 and 62060 not_active; 11032 re-adds 3862 and 7509 while step 62061 is active; completed 62060 and accepted 641 match neither, so no bind returns. Gates read the persisted mission state, so a relog restores exactly the live state. `once` is per player per cell-entity life, re-armed by the rebuild. 11039/11041 handle abandon (status reads `not_active` after the mutation, `lifecycle.rs` 261). |
| G8 Reachability and budget | Pass with risk | The offer is live from 638 completion until 641 accepts (11040/11041 remove it). Prisoners at (-271, -135) and (-254, -135) are 20-30 m from 329 at (-289.8, -113.1), on the walk 638 forces. 640 completes on `teleport_in` region 2 (chain 1044), which is a teleport inside world 12, so the player can walk back (about 70-85 m from ringside Marsh at (-191, -138.6)) unless a barrier blocks it; no static evidence of one, unproven without the lab (CB-U12). Beat: accept click, then aid click on the same prisoner is seconds; sabotage adds the 29 m walk, about 1-2 minutes with dialog reading, inside the 10-minute optional budget shared with 62050 (O14-14, CB-U09 owns the measured baseline). |
| G9 Archetypes | Pass | Jaffa 1018 and Human 1019 both run `complete_mission 638`, which fires `mission_completed 638`; 11030 and 11033/11034 carry no archetype condition. |
| G10 Copy and lore | Pass | F1 is 1369 bytes with SHA-256 `E7F3F137...0439` and F3 is 392 bytes with SHA-256 `5E6316F9...59E7`, both matching the frozen values (recomputed from the file). Text names no 2009 source, uses only "prisoners", "patrol", "guards" and "cell-door panel", fits the title/step/objective/monologue fields, and carries no `CIMMERIA_AUTHORING` (seed comments only). |
| G11 Item | Pass with risk | The executor treats container 0 as unspecified and falls back to `item_container` (`executor/inventory.rs` 52-54). Item 2893 has container sets `{1,17}`; `grant_container` (`cell-catalog/src/item_placement.rs` 69) and `refuse_storage_grant` (`grant/validation.rs` 58-70) keep it out of the bank range 17-20, so it lands in bag 1. A full bag is refused in `grant_item.rs` 454-463 (`GrantRefusal::ContainerFull`): logged as `grant_refused`, no player-visible message, and the chain still completes the mission. CB-N06m checks this in depth. |

## Values

The F5 table is correct as frozen. Checked against the seeds: spawn 520 (-271.0, 65.45, -135.0, tag `Cellblock_Ambient_Prisoner1`, template 176, world 12, stationary), spawn 521 (-254.0, 65.45, -135.0, `..._Prisoner2`), spawn 22 (-287.290009, 67.2600021, -115.040001, tag `329_CellDoorButton`, template 22), Prisoner 329 spawn 6 (-289.808, 65.474, -113.139), maps 3862 and 7509 with NULL `dialog_id` and flags 1073741824, item 2893 sets `{1,17}`, `MAX_INTERACT_DISTANCE` 5.0. `neq` is a supported operator (`content-engine/src/loader/condition.rs` 175), so 11041 loads.

## Changes

None. The architect edits nothing; CB-N06-1..3 copy w5-frozen.md as written.

## Risks

- **CB-U12 walk-back.** Confirm the player can walk from ringside back to the prisoners between 640 and 641 (no barrier after the ring-out). If not, the offer is reachable only between 638 and 640 and the design still works, but the aid/sabotage beat shrinks.
- **Full bag.** Reward loss shows only in the log (`grant_refused`) while the mission completes once; a fresh character has space. Raise with CB-N06m whether a player-visible message is wanted.
- **Accept-on-click wart.** One click accepts and opens 60151 together; closing the dialog does nothing, so a second click on a prisoner is the aid branch. CB-U12 should watch that players read the dialog before clicking again.
- **Shared tags.** `Cellblock_Ambient_*` and `329_CellDoorButton` chains must stay pairwise disjoint (62060 status and step gates); a later packet adding a chain on these tags must keep that.
