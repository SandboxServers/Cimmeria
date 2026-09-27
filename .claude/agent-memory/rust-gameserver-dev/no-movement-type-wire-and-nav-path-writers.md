---
name: no-movement-type-wire-and-nav-path-writers
description: No server-to-client movement-type message exists (method 1 is onSequence, 0x00deb660 is GM onShowPath); NPC stop = zero velocity via npc_ai::movement_stop, the only nav_path writer
metadata:
  type: project
---

Verified in Ghidra on 2026-09-25 (NA10):

- `0x00deb660` ("MovementTypeSwitch") is registered through `CallbackImpl<Event_NetIn_onShowPath>` on `GameProxyPlayer`. It is the GM path visualiser for `SGWGmPlayer.onShowPath`. `0x00deaaf0` is `onShowCommandWaypoints`, and `0x00dec040` is `onDisableShowPath`.
- `setMovementType` only goes client to server (`Event_NetOut_SetMovementType`, with no NetIn twin). A `WitnessEntityMethod` with `method_index: 1` reaches the client as **`onSequence`**, so the old broadcast sent truncated Kismet triggers.
- The client animates NPCs from the `EntityMoved` velocity (`0x00dd1650` → `ApplyTransform 0x00e68a30`, actor +0xf4). To stop an NPC, send zero velocity.

**Why:** the "stale movement type" and "send a stop type" ideas in older docs rest on the misread. Any cover-pose or leash-animation plan that relies on a movement type is dead.

**How to apply:**

- Stop NPCs with `npc_ai::stop_movement_on` / `stop_npc_movement`, reroute them with `replace_nav_path_on`, and teleport them with `snap_npc_to`.
- A source-scan guard in `movement_stop/tests.rs` fails on any `.nav_path.clear()/= /push_back/extend` outside that module.
- `set_ai_state_on` stops the NPC on every real state change. A same-state write does not, so content actions that re-assert a state call `stop_movement_on` explicitly.
- `broadcast_movement_type` is a cache plus a DEBUG log only.
- Before choosing a witness method index, check `cell/client_methods/` for what that index is on the receiving entity type, NOT the cell-method constant with the same number. See [[method-idx-duplicate-table-drift]].
