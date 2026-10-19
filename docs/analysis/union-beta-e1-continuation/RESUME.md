# Union Beta E1 continuation: resume card

> **Last updated:** 2026-10-19 (UB-99) · **Ledger:** [README.md](README.md) · **Index:** [beta-work-packets.md](../union-zone-planning/beta-work-packets.md) · **Reference:** [beta-site-e1.md](../../content/beta-site-e1.md)

## State: code complete, lab pending

Every dispatched packet (B0 to B5) is merged into `uzp-e1`, with review fixes. UB-99 (close-out docs) is done. Nothing has run in the lab or the client.

## What is left

| Item | Who | Note |
|---|---|---|
| UB-90 lab rows UB-U1 to UB-U10 | Orchestrator asks the user first, then `lab-driver` step files | Rows live in [unified-uat.md](../../guides/unified-uat.md#beta_site_evo_1); results go to `uat-results.md` (not yet created) |
| H-LBL | Owner | Label for player-facing CIMMERIA_AUTHORING copy; holds UB-50 |
| U-D07 | Owner | xp and naq for the 17 Beta missions (today 0 and 0, D-UB8); holds UB-50 |
| HELD-B16 / UO-16 | Owner (U-D08, U-D11) | The Asgard end-to-end route is unreachable until Omega delivers an Asgard player at 481 S1760 with item 3399 (D-UB13) |

## Hold and closed

UB-22 and UB-51 are held (D-UB5 A). UB-52 is closed (D-UB7). UB-50 waits on the owner. UB-L (Visit 2 and E2) is research only.

## Follow-ups for other campaigns

- Raise the `seeded_sequences_allocate_past_every_seeded_row` floor from 13869 to 23199.
- Dakara and Omega re-carve their authored screen blocks before authoring (stock screens inside them: see the ledger).
- Check the Castle and Cellblock dialog-patch tables for the UB-DP registration gap.

## Merge

The Beta close-out merges `uzp-e1` onto the Union campaign trunk through the coordinator. Retire the `uzp-ub*` worktrees, branches and test databases on merge.
