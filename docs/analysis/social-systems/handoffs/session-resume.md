# Social Systems: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-27. Companions: [decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: plan written, no packet started

The plan PR adds this ledger, against `main` @ `09a880ba`. Branches are `social/*`; the id block is entity templates **390-399** and spawns **490-499**.

| Packet | Status | PR |
|---|---|---|
| Plan | Review | (this PR) |
| SS-E1, SS-00 | Ready | |
| SS-M1, SS-C1, SS-C2, SS-D1 | BlockedDependency (SS-00) | |
| Everything else | BlockedDependency | |

## Owner decisions

The run proceeds on the PROPOSED defaults (D-SS01). The owner answered all six on 2026-09-27, and each matched the recommendation (see the [README](../README.md#owner-decisions)).

## Things a resuming session must not miss

- **The `onDuelEntities*` landmine** (audit A-41, D-SS25). AoI already sends 152 to make NPCs interactable. No duel packet sends 151 or 153 before SS-E1 D-Q5 is answered.
- **The tell byte** is unknown until ORG-E1 Q5 or SS-E1 C-Q1 answers it. SS-C1 cannot merge without it.
- **Channel constants belong to the organizations campaign** (D-ORG14, D-SS17).
- **Two-client tests are type 11 (wireclient) and do not run in CI** (audit A-60). Every packet also needs a CI-run guard.

## Other campaigns to coordinate with

- **Organizations** (cimmeria-fa, `docs/analysis/organizations/`): D-ORG14, ORG-E1 Q5, ORG-04 and ORG-09 chat, ORG-01's text rules, and ORG-06's `destroy_client_entities`. ORG-04 and ORG-09 expect SS-00's flood limit.
- **Bank / Vault** (cimmeria-97): vault mail aliases (D-SS07) and the inventory move path.
- **Crafting** (cimmeria-af, templates 310-329, spawns 410-429): the inventory move path.
- **Pets** (cimmeria-b5, templates 350-369, spawns 450-469): the hostility gates (SS-D2).
- **Stasis debug hub** (#846, templates 300-304, spawns 400-404): SS-U3's clerk position.

## Resuming

1. `git fetch`, then read the status lines in [work-packets.md](../work-packets.md) and `gh pr list --search "head:social/"`.
2. Worker rules: `%TEMP%\cimmeria-castle\SS-WORKER-RULES.md` (the coordinator writes it from `ORG-WORKER-RULES.md` at launch).
3. Dispatch the next Ready packets per the wave graph.
