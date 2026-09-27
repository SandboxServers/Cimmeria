# Crafting: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-26. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: plan merged (#851); CR-01 (#862) and CR-E1 (#858) merged; Wave 1 writing

| Packet | Status | Branch / PR | Notes |
|---|---|---|---|
| Plan | Integrated | #851 | This ledger |
| CR-01 | Integrated | #862 | Catalog, serializers, parsing, feedback path |
| CR-E1 | Integrated | #858 | Documentation only |
| CR-02 | Review | #864 | No client patch needed (D-CR24); closes #271; #718 is superseded, left for its author |
| CR-E2 | Review | #868 | Blueprint-item mapping (193 resolved), Paradigm Guides already seeded, tools name-only |
| CR-03 + CR-04 | Writing | `craft/cr03-login-sync-spend` | One worker, sequential (shared client-sync helpers) |
| CR-05 | Writing | `craft/cr05-stations-tools` | `cell_to_base.rs` split deferred to its own PR |
| CR-06 to CR-15 | BlockedDependency | | See work-packets.md |
| CR-16 | BlockedDependency | | Waits on Bank BV-01 (#872); tell cimmeria-97 when it starts |

## Owner decisions

All six were answered on 2026-09-26 and are recorded in the [README](../README.md#owner-decisions): ASP 1 at level 1 plus 1 per level; free full respec; Common 5 and Racial Paradigm Guide items; blueprints from Blueprint items and research; stations and Field Tools; reverse-engineer recovery that rises with expertise.

## Coordination

- Coordinator session: cimmeria-af. Campaign kicked off by cimmeria-19 for the owner.
- ID blocks: crafting templates 310-329, spawns 410-429; debug hub (#846) 300-304 / 400-404; Harset 200-299 / 300-399; guilds (cimmeria-fa) 330-349 / 430-449; pets (cimmeria-b5) 350-369 / 450-469.
- Guilds edits `bag_max_slots` and `inventory/move_/mod.rs` in a late vault packet; each side messages the other before touching them.
- Owner rule (2026-09-26): request a Copilot review on every PR, wait for it and address it before squash-merging (see the coordinator memory `feedback_copilot_review_before_merge`).
- Worker rules: `%TEMP%\cimmeria-castle\CRAFT-WORKER-RULES.md`.

## Housekeeping

Campaign worktrees: `craft-plan` (this ledger). Before removing any worktree, delete its `external` junction with `cmd /c rmdir <worktree>\external`, never recursively.
