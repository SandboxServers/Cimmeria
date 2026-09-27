# Organizations: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-27. Companions: [decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: plan written, no packet started

The plan PR adds this ledger and fixes three docs that disagreed with the `.def` and `enumerations.xml`. The coordinator session is cimmeria-19's delegate for the campaign; its id block is entity templates 330-349 and spawns 430-449, and its branches are `org/*`.

| Packet | Status | PR |
|---|---|---|
| Plan | Review | (this PR) |
| ORG-E1 | Writing | |
| ORG-01 | Writing | |
| ORG-02, ORG-03 | BlockedDependency (ORG-01) | |
| ORG-04 to ORG-11 | BlockedDependency | |

## Owner decisions

All three were answered on 2026-09-27, each with the recommendation: D-ORG15 (creation is free, behind a constant), D-ORG16 (the leader alone sets the loot mode) and D-ORG18 (one Squad, one Team and one Command per player). Vault size (D-ORG17) and the treasury cap (D-ORG19) moved to the Bank campaign (cimmeria-97) with the vaults.

## Other campaigns to coordinate with

- **Bank / Vault** (cimmeria-97): consumes the [ORG-API](../work-packets.md#bank-campaign-api-org-api). Message it when ORG-02 and ORG-07 merge.
- **Crafting** (cimmeria-af, templates 310-329, spawns 410-429): no shared files unless inventory changes.
- **Pets** (cimmeria-b5, templates 350-369, spawns 450-469): the org work does not touch entity or AoI creation.
- **Stasis debug hub** (#846, templates 300-304, spawns 400-404): ORG-05 rebases onto it and uses the slots in `docs/content/debug-hub.md`.

## Resuming

1. `git fetch`, then read [work-packets.md](../work-packets.md) status lines and `gh pr list --search "head:org/"`.
2. Worker rules: `%TEMP%\cimmeria-castle\ORG-WORKER-RULES.md`.
3. Dispatch the next Ready packets per the wave graph.
