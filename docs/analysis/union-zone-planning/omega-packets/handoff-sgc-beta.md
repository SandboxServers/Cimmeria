# Union Omega: handoff note for the SGC and Beta ledgers

> **Type:** Reference · **Audience:** Union coordinator, SGC and Beta architects · **Last updated:** 2026-10-18 · **Packet:** [UO-00](UO-00.md)
> **Companions:** [Omega ledger](../omega-work-packets.md), [UO-01 placements worknote](worknotes/UO-01-placements.md), [ID amendment request](id-amendment-request.md), [SGC ledger](../sgc-work-packets.md), [Beta ledger](../beta-work-packets.md), [Dakara ledger](../dakara-work-packets.md)

Omega edits none of these files. The coordinator applies the changes below.

## To SGC

| # | Item | Evidence |
|---|---|---|
| a | **C4 coordinate for US-32's `cross_world_teleport 'Omega_Site'`: x 34.70, y -51.28, z -142.00, heading 3.1409.** It is sealed by UO-01 (`nav_inspect` exit 0). **Stale pin:** three SGC files still carry the gate-5 candidate (34.705, -50.634, -149.092), a point on the gate-5 row (z -149.092), inside the gate's event-horizon volume and 7.1 m from the sealed point. Replace it. | `sgc-work-packets.md:167` (K5 row C4); `sgc-packets/US-32.md:63` (the `cross_world_teleport` row); `sgc-packets/US-00.md:69` |
| b | The old SGC ID list (Omega 12000-12399, 63000-63099, 64000-64199; templates 1700-1739, spawns 15000-15099, point sets 2300-2329, items 9300-9329) is superseded by 63800-63999. Checked 2026-10-18: `sgc-work-packets.md:229` says Omega 63800-63999, `:232` records the supersession, and no stale Omega range appears in any SGC file. | `sgc-work-packets.md:229,232` |
| c | **F21:** no SGC chain may `complete_objective` 5645 or 5646 at S4940. Both are optional (`db/resources/Missions/Seed/mission_objectives.sql:3136,3138`), and the engine completes a mission once no required objective is left open (`crates/cell-content/src/cell/missions/progression.rs:464-506`), so 1655 would complete before Omega runs. | Omega F21 |
| d | D-US11 = D-UO11, the letter (item 6786). Omega works either way; if SGC grants 6786, UO-07 adds one `remove_item 6786`. | `sgc-work-packets.md:303` |
| e | Re-offering 1655 after an abandon is SGC's concern. Omega re-offers only 1423 and 1500 (UO-09). | Omega § Contracts → Receiving |

## To Beta

| # | Item | Evidence |
|---|---|---|
| a | Beta's receiver table cites UO-07 (Human 1423 offer, chains 63820-63822), UO-08 (Free Jaffa 1500 offer, chains 63830-63832) and UO-16 (Asgard, HELD on U-D08/U-D11). Current as of this check. The departure-fixture owner is **UO-13**; Beta builds its own receiver fixtures and does not wait for it. | `beta-work-packets.md:106-114` |
| b | Re-grep `beta-work-packets.md` and `beta-packets/` for `UO-0[2-6]` and for any dependency on UO-13 before relying on them. The 2026-10-18 pass fixed the known stale citations; the remaining `UO-xx` mentions found today (`UB-10-13.md:40,53`, `UB-20-22.md:22,32`, `UB-30-36.md:22,26,48`) cite UO-07, UO-08, UO-13 and UO-16 correctly. | those lines |
| c | Beta's `entity_templates` footer (top 2349) and spawn footers (top 23199) are lower than Omega's block, so they do not mask Omega's floor test (UO-02 revert proof). Not re-verified here: UO-02 merged in #1890, so confirm against its test when Beta's first seed lands. | UO-02 |
| d | Omega's departure states (from § Contracts → Producing): Human leaves 1655 completed and 1423 active at S4223 with gate 6 granted; Free Jaffa leaves 1655 completed and 1500 active at S4369 with gate 6; Asgard (UO-16, HELD) would leave 481 at S1760 with item 3399. Omega fires no Beta step (D-UO2). | `omega-work-packets.md:146-152` |

## To Dakara

`archetype lte 6` in UD-04a admits Asgard (5) and Goa'uld (6) as Tau'ri (`dakara-packets/UD-04a.md:76`). Omega gates Humans with `gte 1` **and** `lte 4` (F20). Dakara decides whether its valve should match.

## Open

| Item | Exact question | Who answers | Blocks |
|---|---|---|---|
| **U-D04** | Does Omega receive 1655 at S4940 with O6055 done (SGC fires `advance_step 4941`, then `4940`, before the teleport)? Does Omega own S4940 → S4942 and the S4942 completion? | owner, with the SGC coordinator | the merge of UO-04, UO-07, UO-08 |
| **U-D07** | What reward and scale may Cimmeria-authored content carry at Omega? How many ambient NPCs (proposed cap 24), and any XP, items or naquadah (proposed none)? | owner | UO-14's merge, UO-15 |

The owner lifted the U-D03 hold (until FZ-99) on 2026-10-18. These two holds remain.

## Documentation debt

- Omega's block collides with stock dialog screen ids 63847-63887; see the [amendment request](id-amendment-request.md#collision-checks). The ID page should say screens and buttons are not block-reserved.