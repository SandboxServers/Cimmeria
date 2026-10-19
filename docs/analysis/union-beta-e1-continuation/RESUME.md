# Union Beta E1 continuation: resume card

> **Last updated:** 2026-10-19 (U-D07 Approved) · **Ledger:** [README.md](README.md) · **Index:** [beta-work-packets.md](../union-zone-planning/beta-work-packets.md) · **Reference:** [beta-site-e1.md](../../content/beta-site-e1.md)

## State: code merged (#1916), lab pending, reward waves ready

Every dispatched packet (B0 to B5) and the UB-99 close-out are merged. Nothing has run in the lab or the client. Owner decisions of 2026-10-19 are in the ledger § Owner decisions: H-LBL (no visible label), U-D08/U-D11 (remote Council hearing, databank prologue issuing item 3399) and **U-D07 Approved 2026-10-19**: the [reward scale](reward-proposal.md) for XP and naq, unchanged, for authored content generally (side activity at 50%). D-UB8 amended (columns stay 0), D-UB16 Decided.

## Next action: wave R1

| Packet | Status | Worktree (from `origin/main`) | Agent | Packet text |
|---|---|---|---|---|
| UB-50a | Ready | `uzp-ub50a` | packet-coder | XP reward chains 23480-23496, [UB-50.md](../union-zone-planning/beta-packets/UB-50.md#ub-50a-beta-mission-xp-via-reward-chains) |
| UB-50b1 | Ready | `uzp-ub50b1` | packet-coder (escalation: rust-gameserver-dev) | `grant_cash` content action, [UB-50.md](../union-zone-planning/beta-packets/UB-50.md#ub-50b1-grant_cash-content-action-code) |
| UB-50b2 | BlockedDependency | `uzp-ub50b2`, cut after R1 merges | packet-coder | naq rows on the same chains, [UB-50.md](../union-zone-planning/beta-packets/UB-50.md#ub-50b2-beta-mission-naquadah-via-the-reward-chains-seed) |

R1: UB-50a and UB-50b1 in parallel (no shared file), one packet-review-fixer for both, each ships as its own PR. R2: UB-50b2 alone after both merge, its own packet-review-fixer.

## Also left

| Item | Who | Note |
|---|---|---|
| UB-90 lab rows UB-U1 to UB-U10, UR1 | Steps written, awaiting owner lab approval. Orchestrator asks the user first | Run order, fixtures and stager flow in [uat/README.md](uat/README.md); grades go in [uat-results.md](uat-results.md). UR1's naq half needs UB-50b2; U5, U5b, U5c, U10 wait on UO-16 |
| HELD-B16 | Omega coordinator (UO-16) | BlockedDependency, not an owner question |
| Omega/Dakara U-D07, H-LBL, U-D08/U-D11 lines | Their coordinators | Exact lines in the ledger § Omega lines still open |

No owner question is open for Beta's packets. UB-22, UB-51 and the UB-50 relays are held (D-UB5 A); UB-52 is closed (D-UB7); UB-L is research only.

## Follow-ups for other campaigns

- Done: the `seeded_sequences_allocate_past_every_seeded_row` floor is already 23199 in `live_db_seed_sequences.rs`.
- Dakara and Omega re-carve their authored screen blocks before authoring (see the ledger).
- Done (UB-F1): the Castle and Cellblock dialog-patch tables are listed in `DIALOG_PATCH_TABLES`, and `the_castle_table_is_registered_for_serving` and `the_cellblock_table_is_registered_for_serving` guard that with revert proofs.
- Whoever implements #310 moves the approved values into `missions.reward_xp`/`reward_naq` and deletes `beta_e1_rewards_chains.sql` in the same change.

## Merge

Each reward packet ships as its own PR through `ops`; retire its worktree, branch and `sgw_<worktree>` database on merge.
