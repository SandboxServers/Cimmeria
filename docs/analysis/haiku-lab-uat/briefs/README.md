# Haiku lab UAT round 1: driver briefs

> Type: reference. Audience: whoever writes the next round's driver briefs or turns these rows into `uat-specs` step files.
> Updated: 2026-10-12. Companions: [round report](../README.md), [evidence ledger](../ledger.md), lab-harness issue [#1660](https://github.com/SandboxServers/Cimmeria/issues/1660).

These are the briefs the 2026-10-12 round gave its Haiku drivers. They are kept as worked examples of the brief format: a chunk of 2-4 steps, its precondition and postcondition, the exact tool calls, and the stop rules. They are not specs. Where a live run proved a brief wrong, the [round report's proven recipes](../README.md#proven-driving-recipes) are what to copy. Each brief takes the lease id as `<LEASE_ID>`, which the coordinator lends to the driver.

| Brief | Steps | Status |
|---|---|---|
| [verify-template.md](verify-template.md) | all | Current. ClickHouse query template the coordinator used to grade each row |
| [bank/BANK-C1.md](bank/BANK-C1.md) | Bank 12, 1, 13 (GM) | Ran as written (after the start-up retry). Its chat macro is reused by every bank brief |
| [bank/BANK-C2.md](bank/BANK-C2.md) | Bank 3, 4, 5, 14 | Partly superseded: `client_move_to` to the vendor, split drags and the vendor quantity spinner all failed. Use teleport-then-click, `setVendorSelectedQuantity` and `moveItem` |
| [bank/BANK-C3.md](bank/BANK-C3.md) | Bank 2, 7, 8, 9, 10 | Step 2 ran as written. Steps 7-10 were superseded by held-key walking (7), BANK-R8 (8, 9) and BANK-R10 (10) |
| [bank/BANK-C4.md](bank/BANK-C4.md) | Bank 6, 11 | Ran as written |
| [bank/BANK-R8.md](bank/BANK-R8.md) | Bank 8, 9 | Re-brief. Ran; step 8 blocked (P5), and the mail half of 9 is an invalid test (D2) |
| [bank/BANK-R10.md](bank/BANK-R10.md) | Bank 10 | Re-brief. Blocked: the Quartermaster's vendor window never opened |
| [consumables/CONS-C1.md](consumables/CONS-C1.md) | I1, I2 | Partly superseded: `/gmsethealth` cannot target yourself (P1), and `client_item_action` sends nothing. Use the HP staging and `useItem` recipes in CONS-R2 |
| [consumables/CONS-C4.md](consumables/CONS-C4.md) | I6, I7, I8 | Ran, with `useItem` (N3) in place of its `client_item_action` USE macro |
| [consumables/CONS-R2.md](consumables/CONS-R2.md) | I3, I4, I5, I9, I10, I10b | Current re-brief. Replaced CONS-C2, CONS-C3 and CONS-C5, which never ran and were dropped. Holds the observed HP and focus staging |
| [abilitytrees/AT-C1.md](abilitytrees/AT-C1.md) | AT 1, 2 | Partly superseded: the camera face loop pinned pitch at the clamp. Add `client_camera {"pitch_counts": -650}` and stand 4-4.5 m from the trainer |
| [abilitytrees/AT-C2.md](abilitytrees/AT-C2.md) | AT 3, 4 | Ran. Step 4 needed the Ability window reopened after the level-up (P2) |
| [abilitytrees/AT-C3.md](abilitytrees/AT-C3.md) | AT 7, 5, 6 | Ran as written |

P and D numbers refer to the round report's [product findings](../README.md#product-findings) and [doc mismatches](../README.md#doc-mismatches).
