---
name: Slash palette evidence for patch-012 entries 28–54
description: Source-checked GM grants, resets, travel, mob ability set and organization reload; native event binding remains unproved
type: reference
---

# Slash entries 28–54 (2026-10-16)

- `data/client-patches/012-gm-slash-commands/commands.toml` has 27 words in this slice. Its optional `Integer abilitySetId` on `/gmsetmobabilityset` conflicts with the implemented cell handler's mandatory `INT32`; `/gmdhd` uses signed `INT8` on the wire and stat target IDs use `INT64`. DW-24B must establish the native parser's quoting, optional/default behavior and emitted method before any action is enabled. Source: `docs/analysis/dev-workbench/evidence/DW-25B-E.md` and `docs/protocol/cell-method-dispatch-table.md`.
- `gmReloadOrganizations` (164) is implemented even though the older guide row says not yet. `crates/cell-console/src/cell/console/gm/organizations.rs::handle_reload_organizations` forwards the caller's identity; `crates/base-session/src/base/organization/handlers/gm_inspect.rs::gm_reload` re-authorizes, reads memberships and re-pushes caller client state, possibly partly failing. It does not reload or mutate organization DB definitions. It is a transient push, not a read-only operation. Source: the cited handlers and `crates/cell-console/src/cell/console/gm/tests/organizations.rs`.
- `server_entity_get` does not expose a mob's known abilities or set ID, so `/gmsetmobabilityset` has no independent live readback in the existing lab tools. Source: `crates/wire/src/cell/messages/lab.rs::LabEntitySnapshot` and `crates/cell-console/src/cell/console/gm/mob_ability_set.rs`.
- Same-space `/gmgoto` and `/gmgotoxyz` conditionally persist the caller's `sgw_player.pos_x/y/z` through `TeleportPlayer`; `/gmsummon` does likewise only for a player target. `/gmgotolocation` and a successful `/gmdhd` go through `GateTravel` and conditionally persist destination world/position plus eligible `known_stargates`. The direct GM address grant in `gm/travel/dhd.rs` is live-memory first, but arrival can make gate IDs durable. Source: `crates/base-world-entry/src/base/world_entry/teleport.rs::handle_teleport_player` and `gate_travel/{mod.rs,persist_arrival.rs}`.
