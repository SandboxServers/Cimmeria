# U-D07 reward scale: xp and naquadah for the 17 Beta missions

> **Type:** Reference (decision record) · **Ledger:** [README.md](README.md) · **Status: U-D07 Approved 2026-10-19** (owner): both the XP and the Naq columns, unchanged; the scale applies to authored content generally (§ 2)
> **Provenance:** every number on this page is **CIMMERIA_RECONSTRUCTION**. None is RESTORED_SOURCE.

The owner asked (2026-10-19) for level-appropriate xp and naquadah for the 17 Beta missions (world 23) and for authored content generally: search for a formula first, and propose a labelled reconstruction if none exists. The architect proposed this page the same day, and the owner approved it unchanged. D-UB8 is amended: the `missions` columns stay 0, and the reward chains pay (§ 4, D-UB16).

## 1. Evidence search: no formula exists

| Source | What it says | Weight |
|---|---|---|
| `db/resources/Missions/Seed/missions.sql` | All 17 Beta missions: `award_xp true`, `reward_xp 0`, `reward_naq 0`, no reward groups (UB-00a2 Q11). The seed is Project Giza's reconstruction, not CME's database | No amounts |
| `docs/analysis/castle-cellblock-rebuild/work-packets.md` GC3, decision D-CB10 | The mission XP formula is **BlockedDesign**. The only observation is 52 XP for Castle missions 680, 681 and 686 (level 1, difficulty 1), from `Mob Stats.txt`, an empirical secondary source. GC3 forbids hardcoding 52 | One data point, one level |
| `docs/analysis/issue-triage-2026-09-25/findings/content.md` (#310) | Mission rewards are unimplemented. Owner options listed there: (a) reverse-engineer the formula, (b) a level x difficulty table as a Cimmeria decision, (c) per-mission `grant_xp` | Option (b) is this page; option (c) is the delivery (§ 4) |
| `docs/gameplay/progression-system.md`, `crates/game/src/player.rs` `LEVEL_XP` | Level bands: L5 400, L6 600, L7 900, L8 1500, L9 2000, L10 3000 XP. Kill XP is 10 x mob level | Scale anchor, not a mission formula |
| `db/resources/Loot/Seed/loot.sql` | Naquadah drops: table 2 5-50 at p0.8, table 4 5-20 at p0.5, table 5 10-35 at p0.6, table 7 5-25 at p0.5, chests 8/9 25-75. Mean per drop over tables 2, 4, 5, 7 is about 12 naq. No level scaling | Weak naq anchor |
| Harset seeds (`harset_opcore_chains.sql:252`, `harset_jaffa_chains.sql:447`, `harset_goauld_chains.sql:595`) | GC3 comments only; no amounts | None |
| Owner memory (four-zone planning) | "Initial rewards are useful supplies and modest XP/currency" | Tone: modest |

No client table, archive or RE note gives a mission xp or naq curve. The answer to "is there formula evidence" is **no**: the proposal below is a reconstruction.

## 2. Approved scale (CIMMERIA_RECONSTRUCTION)

- **XP = 50 x mission level x difficulty factor**, rounded to 5. The factor is 1.0 for difficulty 1 and 1.5 for difficulty 2.
  - 50 x L equals five same-level kills (kill XP is 10 x L).
  - At level 1 it gives 50, within 4% of the one observation (52, GC3). It does not hardcode 52.
- **Naquadah = 10 x mission level x difficulty factor**, rounded to 5. At level 6 that is 60, about five average Castle loot drops. "Modest", per the owner's direction.
- **Authored content generally** (Omega UO-15, Dakara UD-01a/c and later Union campaigns): an authored mission uses the same formula at its own level and difficulty. Authored side activity that is not a mission (a repeatable task, a one-off favour) pays 50% of the story value at that level. The owner approved this general rule with the Beta table (2026-10-19), so a later campaign lists its own table from the formula and needs no further approval for the scale; a departure from the formula is a new owner question. The cross-campaign record is the Shared policy row U-D07 in [union-zone-planning/README.md](../union-zone-planning/README.md#shared-policy-all-union-campaigns).

## 3. Table for the 17 Beta missions

Level and difficulty are from `missions.sql`. "Paid today" says whether Beta's first ship can complete the mission (D-UB5 A stops the tails on Beta, so 1425, 1427 and 1503 have no Beta completion; their values are recorded but stay inert until a later campaign completes them).

| Race | Mission | Name | Level | Diff | XP | Naq | Paid today | Reward chain (UB-50a/b2) |
|---|---|---|---:|---:|---:|---:|---|---|
| Human | 1423 | The SGU | 6 | 1 | 300 | 60 | yes | 23480 |
| Human | 381 | Closing Shop | 6 | 1 | 300 | 60 | yes | 23481 |
| Human | 473 | Inventory | 6 | 1 | 300 | 60 | yes | 23482 |
| Human | 478 | Instant IT | 8 | 1 | 400 | 80 | yes | 23483 |
| Human | 1425 | SGU Database | 8 | 1 | 400 | 80 | no (tail) | 23484 |
| Human | 1426 | Amateur Photography | 8 | 1 | 400 | 80 | yes | 23485 |
| Human | 1427 | The Ruins | 10 | 2 | 750 | 150 | no (tail) | 23486 |
| Free Jaffa | 1500 | The SGU | 5 | 1 | 250 | 50 | yes | 23487 |
| Free Jaffa | 1502 | The Nine Peaks Jaffa | 6 | 1 | 300 | 60 | yes | 23488 |
| Free Jaffa | 1503 | The Antidote | 7 | 1 | 350 | 70 | no (tail) | 23489 |
| Asgard | 481 | From on High | 6 | 1 | 300 | 60 | yes | 23490 |
| Asgard | 420 | Reclamation | 6 | 1 | 300 | 60 | yes | 23491 |
| Asgard | 421 | Mechanical Error | 6 | 1 | 300 | 60 | yes | 23492 |
| Asgard | 1147 | Intriguing Data | 6 | 1 | 300 | 60 | yes | 23493 |
| Asgard | 443 | Incendiary | 7 | 2 | 525 | 105 | yes | 23494 |
| Asgard | 477 | Covert Observations | 6 | 1 | 300 | 60 | yes | 23495 |
| Asgard | 479 | Process Support | 7 | 1 | 350 | 70 | yes | 23496 |

Route totals (all 17 / paid today):

| Route | XP all | XP paid today | Naq all | Naq paid today | Context |
|---|---:|---:|---:|---:|---|
| Human | 2850 | 1700 | 570 | 340 | L6 to L10 takes 5000 XP; Beta's story covers about a third, kills cover the rest |
| Free Jaffa | 900 | 550 | 180 | 110 | L5 band is 400, L6 band 600 |
| Asgard | 2375 | 2375 | 475 | 475 | every Asgard mission completes on Beta |

**Owner approval.** U-D07 Approved 2026-10-19: the XP column and the Naq column, both unchanged. The ledger records it in § Owner decisions. UB-50a (XP) and UB-50b1 (the `grant_cash` action) are Ready; UB-50b2 (naq) follows them ([UB-50.md](../union-zone-planning/beta-packets/UB-50.md)).

## 4. Delivery (D-UB16, Decided 2026-10-19)

Editing `missions.reward_xp` / `reward_naq` does **nothing** today: mission reward dispatch is unimplemented (#310). `chosenRewards` is a stub (`crates/cell-methods/src/cell/cell_methods/player/world/mod.rs:155`), `ON_MISSION_REWARDS_DISPLAY = 127` has no sender, and no code reads the two columns.

- **XP (UB-50a):** one reward chain per mission in a new seed file `beta_e1_rewards_chains.sql`, chains 23480-23496. Trigger `mission_completed '<m>'`, gated on the mission's race (D-UB1) and `mission_status <m> completed`; action `grant_xp {"amount": N}`. The `grant_xp` action exists (`crates/content-engine/src/loader/action.rs`, executor `dispatch.rs` arm `Action::GrantXP`, which logs `Content: granting XP` and sends `CellToBaseMsg::GrantXP`, the same path as kill XP). Because it hangs off `mission_completed`, any completion path pays, and the file can be dropped alone (D-UB14).
- **Leave `reward_xp` and `reward_naq` at 0.** When #310 lands and reads the columns, a seed that sets both the columns and a reward chain pays twice. UB-50a adds a test that the 17 columns stay 0. Whoever implements #310 moves the approved values into the columns and deletes `beta_e1_rewards_chains.sql` in the same change.
- **Naquadah (UB-50b1, UB-50b2):** there is no content action that grants cash. `CellToBaseMsg::GrantCash { entity_id, player_id, amount, gm_feedback_to }` exists (loot and GM use it; base `handle_grant_cash`), so code packet UB-50b1 adds a `grant_cash` action (loader, `Action::GrantCash`, executor module `cash.rs`). Seed packet UB-50b2 then adds one `grant_cash {"amount": N}` row to each reward chain 23480-23496. The fallback, `send_system_mail` with `cash`, arrives as mail, a poor first-press experience, and is not used.
- **No player-facing copy.** The client shows the XP gain itself; nothing here needs a label (H-LBL: none anyway).

The relays part of UB-50 (D-UB5 C) is unaffected and stays on Hold under D-UB5 A. The chain range 23480-23499 was UB-50's whole block; rewards take 23480-23496, so revived relays would need 23497-23499 plus a range the coordinator grants.
