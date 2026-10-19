# Union Beta E1 continuation: resume card

> **Last updated:** 2026-10-19 (owner decisions) · **Ledger:** [README.md](README.md) · **Index:** [beta-work-packets.md](../union-zone-planning/beta-work-packets.md) · **Reference:** [beta-site-e1.md](../../content/beta-site-e1.md)

## State: code merged (#1916), lab pending, rewards proposed

Every dispatched packet (B0 to B5) and the UB-99 close-out are merged. Nothing has run in the lab or the client. Owner decisions of 2026-10-19 are recorded in the ledger § Owner decisions: H-LBL decided (no visible label), U-D08/U-D11 decided (remote Council hearing, databank prologue issuing item 3399), U-D07 answered by a reconstruction **proposal** ([reward-proposal.md](reward-proposal.md)).

## What is left

| Item | Who | Note |
|---|---|---|
| U-D07 approval | Owner | Approve the XP column, the Naq column, or other numbers. Then UB-50a is Ready ([UB-50.md](../union-zone-planning/beta-packets/UB-50.md), worktree `uzp-ub50a` from `origin/main`) |
| UB-50a | packet-coder, then one packet-review-fixer | XP reward chains 23480-23496 on `mission_completed`; `missions.sql` untouched (D-UB16) |
| UB-50b | Architect expands after Naq approval | Needs a `grant_cash` content action (code) plus a seed packet |
| UB-90 lab rows UB-U1 to UB-U10 | Orchestrator asks the user first, then `lab-driver` step files | Rows in [unified-uat.md](../../guides/unified-uat.md#beta_site_evo_1); add a UB-50a XP row once it merges |
| HELD-B16 | Omega coordinator (UO-16) | BlockedDependency, not an owner question: Omega must deliver an Asgard player at 481 S1760 with item 3399 |
| Omega/Dakara line updates | Their coordinators | Exact lines listed in the ledger § Omega lines still open |

## Hold and closed

UB-22, UB-51 and the UB-50 relays are held (D-UB5 A). UB-52 is closed (D-UB7). UB-L (Visit 2 and E2) is research only. CB-N05a does not block UB-50a (no new mission).

## Follow-ups for other campaigns

- Raise the `seeded_sequences_allocate_past_every_seeded_row` floor from 13869 to 23199.
- Dakara and Omega re-carve their authored screen blocks before authoring (stock screens inside them: see the ledger).
- Check the Castle and Cellblock dialog-patch tables for the UB-DP registration gap.
- Whoever implements #310 moves the approved values into `missions.reward_xp`/`reward_naq` and deletes `beta_e1_rewards_chains.sql` in the same change.

## Merge

UB-50a ships as its own PR from `uzp-ub50a` through `ops`; retire its worktree, branch and `sgw_uzp-ub50a` on merge.
