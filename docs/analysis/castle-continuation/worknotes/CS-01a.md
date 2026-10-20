# CS-01a worknote: CS-01 walk step file

> **Type:** Worknote. **Audience:** Castle coordinator, CS-01b/CS-01c runners.
> **Status:** Step file written, never run, no lab use.
> **Companions:** [walk step file](../uat/CS-01-walk.md), [Castle ledger](../README.md).

Base: `FEATURE_SHA c481008904ba39c80168424f67d96b9b271b9ac9`, verified before work.

## What was written

`uat/CS-01-walk.md`: Numbered rows from Cellblock arrival (chain 1109) through missions 701, 702/703, 704, 706 and 708 to the Harset crossing, two relog points (R-1 Zuritska, R-2 DHD), three G-ROUTE walks (W-1 Infirmary to chamber, W-2 Armory to DHD, W-3 Harset to Castle) and three observer rows on `p5`. Every expectation carries `[SEED]`, `[CODE]` or `[INFER]`.

## Gaps (not steps)

| ID | Gap | Effect |
|---|---|---|
| G-01 | No natural route to mission 688 step 80688 in this file; the walk uses `/gmmissionassign 688 1` and `/gmmissionadvance 688 80688` (docs/commands.md:369-370). The refusal rule of `gmMissionAdvance` for that step is unverified. | S-11 may need a different setup. |
| G-02 | No seed or doc gives a walkable route between Copplemann, the Interrogation Block, the Comms Room, the Throne Room and the Armory. | Those rows use `client_move_to` hops plus a recorded GM fallback. |
| G-03 | No combat tool in the driver list; Romney (703) and the Bravo officers / Muelbach (708) kills cannot be driven. | 703 completion and chains 1272/1273, 1346 to 1349 stay unexercised unless the coordinator kills them. |
| G-04 | The Castle Stargate prefab has no spawn row; S-66 aims at the DHD area. | The crossing position is unproven. |
| G-05 | `set_interaction_type` writes a shared actor flag; whether a second player sees the cue is unverified. | S-OBS-2 decides it. |
| G-06 | The driver has no `lab-server` tools, no `client_minigame_play`, no `lab_livewire_plan`, no `client_input_key`, no `lab_route_record`, no character creation. | Minigame rows, character creation and every server check are COORD rows. |
| G-07 | `/gmmissionreset` is documented (docs/commands.md:378) but has no implementation (dispatch table row 117); there is no way to rerun a completed mission. | A rerun needs `/gmmissionclear <missionId>` or a fresh character. |
| G-08 | The Jaffa archetype value 8 in the chains versus the separate `ARCHETYPE_Jaffa` / `ARCHETYPE_Sholva` rows in the archetype seed is unreconciled. | The coordinator confirms the created Jaffa's value (precondition table). |
| G-09 | The playtest finding H10 (DHD `INT_DHD` has no interact arm; dialog-set 3073 NULL dialog) may still be open. | S-65 is written to record either outcome. |
| G-10 | The exact log strings for dialog-open, cross-world teleport, Livewire win, `open_loot` and `player_loaded` were not located. | Rows name them `[INFER]`; the coordinator records the real line. |
| G-11 | `/gmmissionadvance` argument order (`<missionId> <step>`) is documented in docs/commands.md:369 but was not checked by a typed run. | Verify on the first use in S-11. |
| G-12 | `unhandled_interaction_flag` appears only in playtest docs, not in any crate, so no log line can be matched for an unhandled click. | S-65 uses the step read and a screenshot path instead of a log check. |

## Not checked

The `uat/CS-01.md` branch copy was the only prior probe read. Dialog button window names were not looked up: rows tell the driver to read `client_ui_state` first. No chat line, tool call or lab step was run.
