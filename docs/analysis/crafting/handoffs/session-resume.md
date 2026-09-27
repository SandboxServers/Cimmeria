# Crafting: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-27. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: Wave 0 complete; Wave 1 CR-03 to CR-05 integrated (#884, #895), CR-06 in review (#897)

| Packet | Status | Branch / PR | Notes |
|---|---|---|---|
| Plan | Integrated | #851 | This ledger |
| CR-01 | Integrated | #862 | Catalog, serializers, parsing, feedback path |
| CR-E1 | Integrated | #858 | Documentation only |
| CR-02 | Integrated | #864 | No client patch needed (D-CR24); closes #271; #718 is superseded, left for its author |
| CR-E2 | Integrated | #868 | Blueprint-item mapping (193 resolved), Paradigm Guides already seeded, tools name-only |
| Telemetry contract | Integrated | #877 | Work-packets "Telemetry contract" and the CR-14 SigNoz queries |
| CR-03 + CR-04 | Integrated | #884 | Login bundle always carries all five paradigms (a partial stored array is filled with the starting levels at load) |
| CR-05 | Integrated | #895 | 140 is the last message of the login bundle; station state resets at each world entry; craft anywhere is per character. `cell_to_base.rs` split deferred to its own PR |
| CR-06 | Review | #897 | Engine API for the verb packets is in `worknotes/cr-06.md` "Integration edits"; adds the first-player-container helper CR-16 reuses |
| CR-10, CR-12, CR-15 | Ready | | Unblocked by #884; Wave 2 dispatch |
| CR-07 to CR-09 | BlockedDependency | | Unblock when #897 merges |
| CR-11 | BlockedDependency | | Needs CR-15's item rows |
| CR-16 | BlockedDependency | | Waits on Bank BV-01 (#872); tell cimmeria-97 when it starts |

## Owner decisions

All six were answered on 2026-09-26 and are recorded in the [README](../README.md#owner-decisions): ASP 1 at level 1 plus 1 per level; free full respec; Common 5 and Racial Paradigm Guide items; blueprints from Blueprint items and research; stations and Field Tools; reverse-engineer recovery that rises with expertise.

## Coordination

- Coordinator session: cimmeria-23 since 2026-09-27 (cimmeria-af until a system restart ended it). Campaign kicked off by cimmeria-19 for the owner.
- ID blocks: crafting templates 310-329, spawns 410-429; debug hub (#846) 300-304 / 400-404; Harset 200-299 / 300-399; guilds (cimmeria-fa) 330-349 / 430-449; pets (cimmeria-b5) 350-369 / 450-469.
- Guilds edits `bag_max_slots` and `inventory/move_/mod.rs` in a late vault packet; each side messages the other before touching them.
- Owner rule (2026-09-26): telemetry is a first-class deliverable (D-CR27); every packet has a telemetry acceptance line and CR-14 carries the SigNoz queries.
- Owner rule (2026-09-26): request a Copilot review on every PR, wait for it and address it before squash-merging (see the coordinator memory `feedback_copilot_review_before_merge`).
- Worker rules: `%TEMP%\cimmeria-castle\CRAFT-WORKER-RULES.md`.

## Findings outside a packet

- #898: `ConnectedClientState.world_name` is never updated by gate travel (found in the CR-06 review). Crafting does not depend on it: gate travel drops the induction queue explicitly.
- No test races a craft completion against a move, trade or vendor purchase. A vendor purchase whose cost is a crafting component can still deadlock with a completion; Postgres aborts one side, which for the craft is a rollback, "Crafting failed. Nothing was used." and a resync.
- D-CR13's "a queue of at most 10" is implemented as 10 in all, the running induction included.

## Housekeeping

Campaign worktrees: `craft-plan` (this ledger), `cr06` (#897). Retire each with `tools/build-lane/rm-worktree.sh <name>` (PR #869; until it merges, run it from `git show origin/pr869:tools/build-lane/rm-worktree.sh`) the day its PR merges; `cr0304` and `cr05` are retired. Before removing any worktree, delete its `external` junction with `cmd /c rmdir <worktree>\external`, never recursively.
