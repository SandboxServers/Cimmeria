# Organizations: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-27. Companions: [decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: wave 2 in flight

The coordinator session is **cimmeria-1f**. The campaign's id block is entity templates 330-349 and spawns 430-449, and its branches are `org/*`.

| Packet | Status | PR or branch |
|---|---|---|
| ORG-E1 | Integrated | #861 |
| ORG-01 | Integrated | #871 |
| ORG-02 | Integrated | #881 |
| ORG-03 | Integrated | #886 |
| ORG-04 | Integrated | #922 |
| ORG-05 | Writing (dispatched 2026-09-27) | `org/05-creation` |
| ORG-06 | Writing (dispatched 2026-09-27) | `org/06-presence` |
| ORG-07 | BlockedDependency (ORG-05, ORG-06) | |
| ORG-08, ORG-09, ORG-10 | BlockedDependency (ORG-07; ORG-09 also waits for the social campaign's SS-C4) | |
| ORG-11, ORG-UAT | BlockedDependency | |

### How PRs merge

Copilot reviews are suspended (D-ORG25), so the coordinator reviews each diff itself. `main` moves fast, and PRs have needed one to three rebases each. Once CI is green and the merge state is `CLEAN`, merge against the exact head you reviewed:

```bash
gh pr merge <N> --squash --match-head-commit <sha>
```

Retire the worker's worktree the day its PR merges: `bash tools/build-lane/rm-worktree.sh <name>`.

## Owner decisions

All three were answered on 2026-09-27, each with the recommendation: D-ORG15 (creation is free, behind a constant), D-ORG16 (the leader alone sets the loot mode) and D-ORG18 (one Squad, one Team and one Command per player). Vault size (D-ORG17) and the treasury cap (D-ORG19) moved to the Bank campaign (cimmeria-79) with the vaults.

## Other campaigns to coordinate with

- **Social** (cimmeria-3d): owns channel-id alignment in SS-C4 (D-ORG26), which ORG-09 waits for.
- **Bank / Vault** (cimmeria-79, formerly cimmeria-97): consumes the [ORG-API](../work-packets.md#bank-campaign-api-org-api). ORG-02 has merged; message it again when ORG-07 merges. Its BV-08 owns the CM 19 arm, and its BV-07 closes an open org vault window on `onOrganizationLeft` [36].
- **Crafting** (cimmeria-23, templates 310-329, spawns 410-429): no shared files unless inventory changes.
- **Pets** (cimmeria-b5, templates 350-369, spawns 450-469): the org work does not touch entity or AoI creation.
- **Black market and the stasis debug hub** (cimmeria-11; hub #846 merged, templates 300-304, spawns 400-404): ORG-05 uses the slots in `docs/content/debug-hub.md`.

## Resuming

1. Check `~/.claude/sessions/*.json` for a live coordinator holding `org/*` branches; if cimmeria-1f is still running, message it instead of taking over.
2. `git fetch`, then read the [work-packets.md](../work-packets.md) status lines, `gh pr list --search "head:org/"` and `git worktree list` (the ORG-05 and ORG-06 workers run in `.claude/worktrees/org-05` and `org-06`).
3. Worker rules: `%TEMP%\cimmeria-castle\ORG-WORKER-RULES.md`.
4. Review and merge any green PR as [above](#how-prs-merge), retire its worktree, and update this table.
5. Dispatch the next Ready packets per the wave graph. ORG-07 starts when ORG-05 and ORG-06 are both Integrated; ORG-09 also needs SS-C4.
