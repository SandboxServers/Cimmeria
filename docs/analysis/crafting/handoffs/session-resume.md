# Crafting: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-26. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: plan written, Wave 0 dispatching

| Packet | Status | Branch / PR | Notes |
|---|---|---|---|
| Plan | Review | `docs/crafting-campaign-plan` | This ledger |
| CR-01 | Ready | | Bottleneck; merge first |
| CR-E1 | Ready | | Documentation only |
| CR-02 | Ready | | May stop with a report if the clock needs a client patch |
| CR-03 to CR-12 | BlockedDependency / BlockedDecision | | See work-packets.md |

## Open owner decisions

D-CR01 (earning ASP), D-CR02 (respec cost and scope), D-CR03 (paradigm levels), D-CR04 (blueprint acquisition), D-CR05 (stations and tools), D-CR06 (reverse-engineer recovery). The questions and recommendations are in the [README](../README.md#owner-decisions).

## Coordination

- Coordinator session: cimmeria-af. Campaign kicked off by cimmeria-19 for the owner.
- ID blocks: crafting templates 310-329, spawns 410-429; debug hub (#846) 300-304 / 400-404; Harset 200-299 / 300-399; guilds (cimmeria-fa) 330-349 / 430-449; pets (cimmeria-b5) 350-369 / 450-469.
- Guilds edits `bag_max_slots` and `inventory/move_/mod.rs` in a late vault packet; each side messages the other before touching them.
- Worker rules: `%TEMP%\cimmeria-castle\CRAFT-WORKER-RULES.md`.

## Housekeeping

Campaign worktrees: `craft-plan` (this ledger). Before removing any worktree, delete its `external` junction with `cmd /c rmdir <worktree>\external`, never recursively.
