# Organizations: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-27. Companions: [decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: plan written, no packet started

The plan PR adds this ledger and fixes three docs that disagreed with the `.def` and `enumerations.xml`. The coordinator session is cimmeria-19's delegate for the campaign; its id block is entity templates 330-349 and spawns 430-449, and its branches are `org/*`.

| Packet | Status | PR |
|---|---|---|
| Plan | Review | (this PR) |
| ORG-E1 | Ready | |
| ORG-01 | Ready | |
| ORG-02, ORG-03 | BlockedDependency (ORG-01) | |
| ORG-04 to ORG-11 | BlockedDependency | |

## Open owner decisions

Asked on 2026-09-27; see [README.md § Decisions](../README.md#decisions).

1. **D-ORG15**, creation cost. Recommended: free, behind a constant. Blocks nothing (ORG-05 ships the constant at 0).
2. **D-ORG16**, who sets the squad loot mode. Recommended: the leader only, with visible feedback. ORG-03 ships the recommendation.
3. **D-ORG18**, Team and Command exclusivity. Recommended: not exclusive, one of each type. ORG-02 enforces the recommendation in the schema.

Vault size (D-ORG17) and the treasury cap (D-ORG19) moved to the Bank campaign (cimmeria-97) with the vaults.

## Other campaigns to coordinate with

- **Bank / Vault** (cimmeria-97): consumes the [ORG-API](../work-packets.md#bank-campaign-api-org-api). Message it when ORG-02 and ORG-07 merge.
- **Crafting** (cimmeria-af, templates 310-329, spawns 410-429): no shared files unless inventory changes.
- **Pets** (cimmeria-b5, templates 350-369, spawns 450-469): the org work does not touch entity or AoI creation.
- **Stasis debug hub** (#846, templates 300-304, spawns 400-404): ORG-05 rebases onto it and uses the slots in `docs/content/debug-hub.md`.

## Resuming

1. `git fetch`, then read [work-packets.md](../work-packets.md) status lines and `gh pr list --search "head:org/"`.
2. Worker rules: `%TEMP%\cimmeria-castle\ORG-WORKER-RULES.md`.
3. Dispatch the next Ready packets per the wave graph.
