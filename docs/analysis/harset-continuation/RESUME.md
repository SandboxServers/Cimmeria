# Harset continuation resume

> Type: reference. Audience: campaign coordinator and reviewers. Updated: 2026-10-19.
> Companions: [ledger](README.md), [packet gates](work-packets.md), [O18 register](worknotes/owner-decisions-2026-10-18.md), [Lomiada brief](lomiada-brief.md), [execution contract](../four-zone-planning/execution-contract.md).

- Branch `content/harset-campaign` in worktree `zone-harset`. Evidence base `d3efb7d41`; `origin/main` merged 2026-10-14 (`817d3792c`); CB-N05a integrated by `9b4399ae8`.
- Integrated and reviewed: HA-00b (`37ffd858d` + `22982c17a`, docs only); HA-1360 a1, a2a, a2b, a3a, a3b, a3c (`e7dd74336`), a2c (`41d099ef5`), b1 (`e1c03b107`), b2 (`9455f2af2`); Castle CS-08a1 crossing carried as `d2a6913cd`. Detail: [HA-1360a seal](worknotes/HA-1360a-seal.md), [HA-1360b seal](worknotes/HA-1360b-seal.md), [a2c seal](worknotes/HA-1360a2c-seal.md).
- Docs-only work done: HA-00c static half (`2785df721`, zero collisions, no IDs), HA-15 audit (`0f0ce57c2`), offline nav revalidation, BSP door probe and the nine blank sign-off cards (`59d5d6f0c`), owner decisions O18-01 to O18-13 (`dd70eeafb`).
- **Owner-manual UAT pending** for HA-1360a/b (Cellblock → Castle → Marsh letter). Ask the user and wait for a fresh yes before any lab lease, client start or server retarget. No Harset client or lab result is recorded.

## Next action (2026-10-18)

1. R1 and R2 research DONE and committed (`caafb2e9f`, eight worknotes in [worknotes/](worknotes/): HA-01b1, HA-01b2, HA-03a, HA-15d, HA-15e, HA-17a1, HA-01a1, HA-15f). All designs are PROVISIONAL; nothing tested in game. Reviewed and fixed in the same commit as this file (report `.claude/reports/harset-continuation/r1r2-review.md`, not in git).
2. Done since: HA-03b contract and HA-15g placement plan (`4c9af5f34`); HA-03c `remove_effect` arm integrated (merge `98b9b8741`, review ship). Status table refreshed. HA-03c worktree `harset-ha-03c` awaits `rm-worktree`.
2b. Merged: `content/harset-campaign` head `1f935687c` is an ancestor of `campaign/four-praxis-zone-restoration` (merge `4e0e8a094`); nothing was lost. The earlier do-not-merge instruction no longer applies.
3. Wave findings to carry: 5225 door is server-only feasible on paper (advance_step 5226, never complete_objective 6060; walk-in guard is an owner choice); HA-02 note's world label is a typo (data is Market); Market landing 1551 (20.0,4.81,8.5) and Storage (61.4,5.12,5.2) pass; event 5 consumes nothing for the seven items; no cash action in the chain loader.
4. Remaining offline candidates (each still gated): HA-01b graph seal, HA-15b stock/price seal, HA-03d seed graph, HA-15g trainer trees. PROVISIONAL offline drafts now exist for the first, second and fourth (PU-05, PU-08, PU-07) and for the H05 contract (PU-06), awaiting merge into the combined branch; see the README integration record.
5. The coordinator relays O18-08 (618/621 provisional) to Tollana and O18-01/02/09/12 to Castle. The shared four-zone contract is not edited here.

## Still blocked

- Owner: HA-15a treat (4512 or 2714, bought or granted); prices inside O14-12 (D-PU2); whether the HA-17a2 pilot waits for the Jaffa-arc UAT and Cellblock's first authored proof; the 1664 abandon path without 1399 (O18-02, PU-05 proposal); whether 1664 ships in the PU-05 PR (D-PU1, recommend no).
- Maintainer: vendor faction gating for HA-15b (flavour-only for the first seal, per the PU-08 draft). Full list: [What stays open](worknotes/owner-decisions-2026-10-18.md#what-stays-open).
- Maintainer: the 5225 right-click door, only if HA-01b1 finds no server-only path.
- Evidence: the Market exit arch (1006 or 1551), the world-57 door positions, spawn 308 and the raised points, containment walk, 567 ships enabled under FZ-12 (D-FZ1), with the owner's lab pass (FZ-U1) still open; the Castle 2698 grant (chain 1397) and cross-world replay exist offline; first authored client proof (HA-00c).
- Deferred: Agnos (HA-06c, HA-13c theme leg).
- HA-17a3, HA-17b/c and HA-18a/b/c wait on the pilot's owner-run client check.

## Worktrees awaiting retirement

- Use `rm-worktree.ps1` only; never `git worktree prune`. Under `.claude/worktrees/`: `harset-ha-1360b`, `-1360b1`, `-1360b2`, `-1360a2c`, `-1360a3c`, `-1360a3c-revert`, `-1360a1`, `-1360a2a`, `-1360a2b`, `-1360a2b-refusal`, `-1360a3a`, `-1360a3b`. All are integrated or patch-equivalent.
- **Exception:** `harset-ha-1360a2b-refusal` has two uncommitted files (`letter_turn_in_route.rs`, `letter_turn_in_route_tests.rs`); the owner inspects or discards them first.
- Measurement debt: actual tokens and elapsed check/review time are still unrecorded.
