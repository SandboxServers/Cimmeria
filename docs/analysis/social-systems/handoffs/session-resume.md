# Social Systems: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-27. Companions: [decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: Wave 1 nearly done; Wave 2 writing

Updated 2026-09-27. Branches are `social/*`; the id block is entity templates **390-399** and spawns **490-499**.

| Packet | Status | PR |
|---|---|---|
| Plan | Integrated | #873 |
| SS-E1 | Integrated | #875 |
| SS-00 | Integrated | #880 |
| `chat.rs` split | Integrated | #885 |
| SS-C2 | Integrated | #887 |
| SS-D1 | Integrated | #888 |
| SS-M1 | Integrated | #894 |
| SS-C1 | Review (it also wires SS-M1's and SS-D1's Ignore seams) | #893 |
| SS-M2, SS-D2, SS-U2 | Writing | |
| SS-C3 | BlockedDependency (SS-C1) | |
| Everything else | BlockedDependency | |

Contract changes since the plan are recorded in [work-packets.md](../work-packets.md): `MailOp::SendRejected`, `CellToBaseMsg::Chat(ChatCellToBase)`, and `combat::player_may_attack` (from the pets campaign) replacing `player_may_harm`.

## Owner decisions

The run proceeds on the PROPOSED defaults (D-SS01). The owner answered all six on 2026-09-27, and each matched the recommendation (see the [README](../README.md#owner-decisions)).

## Things a resuming session must not miss

- **The `onDuelEntities*` hold is lifted** (D-SS25 superseded by SS-E1 D-Q5). 151, 152 and 153 edit a set at `GamePlayer+0x16c` that the client's interactability check never reads, so SS-D2 may send 151 at duel start and 153 at duel end. The comment at `aoi.rs:203-211` has 152's direction backwards; SS-D2 fixes it.
- **The PvP-flag vehicle is open** (D-SS23). `pvpFlag` (CELL_PUBLIC, ghosting-only in SGW) and `GENERICPROPERTY_PvPFlag` are both candidates; SS-D2 traces which one the client receives. The combat gate never reads a flag.
- **`MessageAttachment.durability` stays INT32** per `alias.xml` until a capture settles SS-E1's float-read observation.
- **The tell byte** is 10, from ORG-E1 Q5, which SS-E1 C-Q1 cites. The channel constants still belong to the organizations campaign.
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
