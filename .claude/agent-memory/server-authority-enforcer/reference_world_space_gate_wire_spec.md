---
name: reference-world-space-gate-wire-spec
description: Wire shapes + dispatch indices + Ghidra anchors for CAT-O surface (gate / ring / region / world-instance / system-options / movie)
metadata:
  type: reference
---

Authoritative wire shapes and dispatch indices for the CAT-O surface,
cross-referenced to entity defs, Ghidra symbols, and Rust handler files.

## Cell method dispatch indices (client → server)

| Method | Index | Def location | Rust dispatch |
|---|---|---|---|
| `onDialGate` | 35 | `entities/defs/interfaces/GateTravel.def:70-74` (INT32 target, INT32 source) | `crates/services/src/cell/cell_methods/gate_travel.rs:17-37` → `crates/services/src/cell/gate_travel.rs:35-108` |
| `setRingTransporterDestination` | 91 | `SGWPlayer.def:848-852` (INT32 regionId, INT32 destinationId) | `crates/services/src/cell/cell_methods/player/world/mod.rs:207-228` → `crates/services/src/cell/ring_transport/runtime.rs:244-350` |
| `onWorldInstanceReset` | 92 | `SGWPlayer.def:868-870` (no args, `<Exposed/>`) | `crates/services/src/cell/cell_methods/player/world/mod.rs:230-233` — **UNIMPLEMENTED STUB** |
| `updateSystemOptions` | 93 | `SGWPlayer.def:872-875` (ARRAY of NameValuePair) | `crates/services/src/cell/cell_methods/player/world/mod.rs:235-238` → `handle_update_system_options` at lines 262-351 |
| `triggerClientHintedGenericRegion` | 85 | `SGWPlayer.def:766-771` (INT32 id, UINT8 bEntering, VECTOR3 position) | `crates/services/src/cell/cell_methods/player/world/mod.rs:128-191` |
| `cancelMovie` | 108 | `SGWPlayer.def:1104-1107` (WSTRING MovieName) | early-handled in `crates/services/src/base/connect_loop/cell_arms.rs:113-117` → `handle_cancel_movie` at `crates/services/src/base/world_entry_appearance.rs:721-741` |
| `onStrikeTeamResponse` | 11 (org range) | (per `Organization.def` — not searched here) | `crates/services/src/cell/cell_methods/organization.rs:65-77` — **UNIMPLEMENTED STUB** |

## Stub-only (no server arm, falls through to "Unhandled cell method" warn)

| Method | Def | Status |
|---|---|---|
| `onSpaceQueuedResponse` | `SGWPlayer.def:524-527` (INT8 aAccept) | Not in dispatch |
| `onSpaceQueueReadyResponse` | `SGWPlayer.def:519-522` (INT8 aAccept) | Not in dispatch |
| `onSpaceQueueStatus` | `SGWPlayer.def:515-517` (no args) | Not in dispatch |

## Server-side data sources

| Quantity | Authority source |
|---|---|
| Stargate position / destination world | `space_mgr.stargates[address_id]` (loaded from `resources.stargates` at startup) — server-only |
| Player's unlocked stargates | `sgw_player.known_stargates` (Postgres column) → `PlayerLoadData::known_stargates` (`crates/services/src/base/world_entry/methods/player_load/core.rs:60,217`) → shipped to client via `setupStargateInfo` at `crates/services/src/mercury/world_data/map_loaded.rs:175-181`. **NOT plumbed onto `CellEntity`** — gate handler can't consult it without an additional plumb. |
| Ring-transporter pad layout | `space_mgr.ring_transporters` (loaded from DB at world load) |
| Ring mission-gate | `RingTransporter::required_mission_id` (per pad) — checked in `handle_select_destination` at `runtime.rs:277-288` |
| Player position | `space_mgr.get_entity(entity_id).position` — NOT consulted in gate / ring / region handlers (the trust gap in CAT-B-02/03/04) |
| System-options apply allowlist | `crates/entity/src/cell_entity/system_options.rs:59-71` — closed match on `autoReload`, `reloadOnActivate` |
| `ConnectedClientState.access_level` | Per-session GM bit, set at auth. **NOT plumbed into cell-method dispatch** (see [[reference_gm_auth_plumbing_gap]]). |

## Ghidra anchors

| Symbol | Address | Notes |
|---|---|---|
| `Event_NetOut_onDialGate` (string) | 019be588, 019ca724 | |
| `register_NetOut_onDialGate` | 00d93060 | |
| `SGWNetworkManager_VEvent_NetOut_onDialGate___EventHandler__vfunc_0` | 00d69030 | |
| `Event_NetOut_DHD` (string) | 019be1c4, 019ca2b8 | |
| `register_NetOut_DHD` | 00d8fe80 | Same server-side cell-method 35 as onDialGate |
| `Event_NetOut_WorldInstanceReset` (string) | 019b4340 | |
| `register_NetOut_WorldInstanceReset` | 00cbe5a0 | |
| `Event_SlashCmd_WorldInstanceReset` (string) | 018429f4 | C++ client emits from slash-command — no client-side gate |
| `Event_SlashCmd_DHD` (string) | 01841b98 | Same — slash-command path emits through `Event_NetOut_DHD` |
| `Event_NetOut_SetRingTransporterDestination` cluster | 019b3ec8 area | (CAT-B-03 anchor) |
| `Event_NetOut_TriggerClientHintedGenericRegion` | (standard NetOut shape) | (CAT-B-04 anchor) |

## Trust-violation patterns specific to CAT-O

1. **Destination authorization** — independent of source-position trust.
   `handle_dial_gate` checks `target_address_id in space_mgr.stargates`
   (any-world global catalog) but not `target_address_id in
   player.known_stargates`. The destination-authorization fix and the
   CAT-B-02 source-position fix must both ship; either alone is insufficient.

2. **Chain-trigger debouncing** — `triggerClientHintedGenericRegion` has
   no rate-limit or per-region debounce. Even after CAT-B-04's position
   validation ships, a client at the right position can still rapid-fire
   enter/exit across multiple regions to walk the chain engine through
   mission steps faster than the design allows.

3. **GM/player exposed-method confusion** — `onWorldInstanceReset` is on
   `SGWPlayer.def`, not `SGWGmPlayer.def`. The naming implies GM but the
   wire surface is fully exposed. Future implementer must add an explicit
   `is_gm_session()` check; the dispatch layer won't enforce it for them.

4. **Argument-discarded "safe today" handlers** — `cancelMovie` reads no
   args from the payload, `updateSystemOptions` accepts only an allowlist
   of 2 names. Both surfaces *will* become exploitable the moment any
   contributor (a) ships a mission-gating cinematic that should not be
   cancellable, or (b) adds a server-authoritative option to the apply
   match. Comments in the def + handler should call this out by name.

Links: [[reference_gm_auth_plumbing_gap]],
[[reference_cell_method_entity_id_authority]],
[[project_world_space_gate_audit_2026-05-31]].
