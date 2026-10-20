# Union Beta E1 continuation: resume card
> **Last updated:** 2026-10-19 (restoration end state) · **Ledger:** [README.md](README.md) · **Index:** [beta-work-packets.md](../union-zone-planning/beta-work-packets.md) · **Reference:** [beta-site-e1.md](../../content/beta-site-e1.md)
## State: code complete, lab pending
**Release readiness: code complete; ship after the PR merges; UB-U1..U10 lab run is the remaining gate and needs owner approval.**
## Done
| Item | What landed |
|---|---|
| B0 to B5, UB-99 | Every dispatched packet and the close-out, merged in #1916 |
| UB-50a | XP reward chains 23480-23496 (`beta_e1_rewards_chains.sql`) |
| UB-50b1 | The `grant_cash` content action |
| UB-50b2 | Naquadah rows: one `grant_cash` row per reward chain |
| UB-F1 | Castle and Cellblock registration guards in `DIALOG_PATCH_TABLES`; stale 13869 references fixed |
| Decisions | H-LBL (no visible label), U-D07 (reward scale approved, XP and naq, applies to authored content), U-D08/U-D11 (README direction): all Decided |

UB-50a, UB-50b1, UB-50b2 and UB-F1 are Done and Integrated in one PR, opened by the orchestrator from `union-beta/integration`. The earlier PRs #1939 and #1940 are superseded by it and will be closed.
## Tested and never tested
Tested, per `.claude/reports/union-beta-e1/integration-verify.md`: unit (nextest over four crates, 2075 passed), chain-replay (the reward chains per mission) and live-DB tiers (`beta_e1` 208 tests; the whole tier 2286 passed). Never tested: anything in the lab or the game client, including whether a player sees the XP and naquadah.

Known flake: `script_session_live_db_sees_the_arrival_dialog` (1 s ACK window) failed once on CI and passed on rerun.

## Held
| Item | Why |
|---|---|
| UB-22, UB-51 | Held (D-UB5 A): tails stop on Beta |
| UB-50 relays | Held (D-UB5 A); H-LBL means no label if ever revived |
| UB-52 | Closed (D-UB7) |
| UB-L | Research only |
| HELD-B16 | BlockedDependency on Omega UO-16 (a dependency, not an owner question) |

## Next
1. The orchestrator opens the single PR and closes #1939 and #1940. Retire the worktree, branch and `sgw_<worktree>` database on merge.
2. UB-90 is UATPending: steps are written and reviewed in [uat/](uat/README.md), [uat-results.md](uat-results.md) is blank. Ask the user for lab approval first, then run UB-U1 to UB-U10 and UR1.
3. The Asgard rows (U5, U5b, U5c, U10 and the Asgard halves of U1b and U9) are not runnable until Omega UO-16.

## Follow-ups for other campaigns
- #310 must move the reward values into `missions.reward_xp` / `reward_naq` and delete the reward chains in the same change.
- A future campaign that seeds its own `grant_cash` must update `only_reward_chains_grant_cash` in the same change.
- Omega and Dakara coordinators update their U-D07, H-LBL and U-D08/U-D11 lines (exact lines in the ledger § Omega lines still open).
- Dakara and Omega re-carve their authored screen blocks before authoring (see the ledger).