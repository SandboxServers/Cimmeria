# Union Beta E1: Omega receiver contract (frozen)

> **Type:** Reference
> **Audience:** Beta packet coders (UB-10, UB-20, UB-30), Omega packet coders (UO-07, UO-08, UO-13, UO-16) and their reviewers
> **Last updated:** 2026-10-18 (UB-00c)
> **Source:** [beta-work-packets.md § Omega receiver contract](../union-zone-planning/beta-work-packets.md#omega-receiver-contract) and § Findings F17, frozen here at the ids of Omega's 2026-10-18 renumbering
> **Companions:** [README.md](README.md), [Omega packets](../union-zone-planning/omega-work-packets.md), [execution contract](../union-zone-planning/execution-contract.md)

This file freezes the table Beta consumes. Change a row only by changing both sides and this file in the same review; a drift shows up as a failing test on whichever side changes first.

## Contract table

| Race | Omega producer | State on arrival at Beta | Beta's receiving chain | Beta's first talk step |
|---|---|---|---|---|
| Human | UO-07 (Jackson 4641, chains 63820-63822) | 1655 completed; 1423 accepted; S4223 active; gate 6 granted | 23020: `advance_step 1423 -> 4224` | S4224 Carter, dialog 4643 (UB-10) |
| Free Jaffa | UO-08 (Bra'tac 5025, chains 63830-63832) | 1655 completed; 1500 accepted; S4369 active; gate 6 granted | 23220: `advance_step 1500 -> 4370` | S4370 Carter, dialog 5026 (UB-20) |
| Asgard | UO-16 (stub, **HELD**: U-D08/U-D11; the Asgard producer, Thor 1913/1912 and item 3399, does not exist yet) | 481 accepted; S1760 active; item 3399 held. Beta's side is built against this state; no Omega packet delivers it. | 23310: `advance_step 481 -> 1761` | S1761 Carter, dialog 1915 (UB-30) |

Beta fires the arrival step (D-UB2, confirmed by Omega D-UO2): on `player_loaded Beta_Site_Evo_1`, gated on that step being active, and idempotent across relog. Omega departs the player with the step active and fires nothing for it.

**HELD, exact question (HELD-B16).** Which Omega packet delivers an Asgard player to Beta with 481 active at S1760 and item 3399 held? UO-16 waits on the owner's U-D08/U-D11. Until it is answered UB-30 can be coded and replay-tested with archetype 5 (D-UB13), but it is not end-to-end reachable and does not count as route completion.

## Fixture ownership

| Fixture | Owner | Where | What it asserts |
|---|---|---|---|
| `human_departure`, `jaffa_departure` | Omega **UO-13** | `chain_replay_tests/union_omega/handoff.rs` | Omega's exact output state (the "departure" side) |
| `chain_replay_tests::live_db_beta_e1::m1423::receiver_*` | Beta **UB-10** | live-DB test | stages the Human row of the table, asserts chain 23020 resolves to S4224 |
| `chain_replay_tests::live_db_beta_e1::m1500::receiver_*` | Beta **UB-20** | live-DB test | stages the Free Jaffa row, asserts chain 23220 resolves to S4370 |
| `chain_replay_tests::live_db_beta_e1::m481::receiver_*` | Beta **UB-30** | live-DB test | stages the Asgard row (archetype 5), asserts chain 23310 resolves to S1761; HELD for reachability, not for the test |

Beta's wave B2 can run before UO-13 (Omega W4) merges, so Beta stages the state itself and never imports `handoff`. It may switch to the import once UO-13 is on `main`. Both sides assert the same table row.

## Mismatches M1-M9

Mismatches between the Beta plan and Omega's contract, from the index § Findings F17 and the contract section. Each has a Beta-side resolution; none needs an Omega change except where stated.

| # | Mismatch | Resolution (Beta side) | Omega action |
|---|---|---|---|
| M1 | Who fires the arrival steps 1423 S4223, 1500 S4369, 481 S1760 | Beta, on `player_loaded Beta_Site_Evo_1` (D-UB2, Decided) | none; Omega D-UO2 already fires no Beta step |
| M2 | Dialog 4643 uses the mid-step flag 268435456 | Beta displays the dialog from a chain, so the flag only changes the indicator glyph; cosmetic. UB-00a2 Q9 confirms or refutes | none |
| M3 | **Not defined** in the planning folder | The index names M1-M9 but gives no M3 row. Left open here rather than invented | architect to define or retire the label |
| M4 | Beta must not re-offer 481 (nor 1423, 1500) | Beta never offers them (D-UB3, Decided); Omega owns the offers | none |
| M5 | **Not defined** in the planning folder | as M3 | architect to define or retire the label |
| M6 | Heimdall S4381 (1503 tail) | Offworld tail; the arc stops at its last Beta-local step (D-UB5 A, default). Owner picks B or C otherwise | only under D-UB5 B |
| M7 | 446 S1496/S1497 | 446 is LATER content; never on Beta past S5030 | none |
| M8 | 1425 S4244 (Jackson) and 1427 S4250 (Woolsey) | Offworld tails, as M6 (HELD-B4) | only under D-UB5 B (Omega Jackson, SGC Woolsey) |
| M9 | S4224, S4370 and S1761 are Beta-owned | The receiving chains 23020, 23220 and 23310 advance into them; Omega never fires them | none |

Related open question, not an M-row: HELD-B1b, which 481 task consumes item 3399. Beta removes the item at S1761 unless UB-00a2's seal says otherwise; the Omega architect confirms.

## Stop condition

A race that receives another race's `The SGU` mission, or completes another race's step, stops the Beta work (index § Stop conditions). The disjointness of the 1655 guards on the Omega side (`mission_status 1655 eq active` on delivery, `eq completed` on re-offer) is Omega's test; Beta's cross-race guard is UB-45a.
