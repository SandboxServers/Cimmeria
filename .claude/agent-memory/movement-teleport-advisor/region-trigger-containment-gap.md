---
name: region-trigger-containment-gap
description: Block on sight — Cimmeria's triggerClientHintedGenericRegion arm never re-checks that the entity is actually inside the region; 2009 did, and any region routing that moves a player turns this into a free teleport
metadata:
  type: project
---

**Block on sight: routing a client-hinted region trigger to anything that moves, destroys or transfers the player, without first re-checking server-known containment.**

2009 `deprecated/python/cell/GenericRegion.py::triggerRegion` gates every entering
trigger:

```python
if entering and not region.isPointInRegion(entity.position):
    warn(... "attempted to trigger generic region <%d> from too far away")
    return
```

It tests `entity.position` — the **server-known** position — not the position the
client sent in the RPC args.

Cimmeria does not have this. `crates/services/src/cell/cell_methods/player/world/mod.rs`
TRIGGER_REGION arm parses the client's `x/y/z` into `_x/_y/_z` and discards them,
then fires `fire_enter_region` / `fire_exit_region` and forwards to the ring FSM
with no containment test of any kind. There is no `is_point_in_region` helper
anywhere in `crates/` (only `SpaceManager::get_region`, `space_manager/queries.rs:190`).

Today that is a content-trigger annoyance. It becomes a **teleport exploit** the
moment a region routes to travel — e.g. Harset H01's `REGION_FLAG_STARGATE`
(bit 2) routing, where `triggerClientHintedGenericRegion(1001, true, …)` from
anywhere in the world would fire gate passage.

The bbox needed for the check already exists and is the *same shape the client
was sent*, so server and client agree: `crates/cell-catalog/src/cell/spawner/regions.rs:92-101`
expands a single-point cylinder into 4 corners, `X/Z within ±radius`, `Y within
[py, py+h]`. **Y is the vertical axis** (see [[arrival-coordinate-offnavmesh]]).
The fourth corner's `py + h` asymmetry is deliberate 2009 parity — do not
"normalize" it.

**How to apply:** port the containment gate scoped to the arm that moves the
player first (widening it to the content and ring arms is a behavior change that
could break authored volumes — hand that to server-authority-enforcer). Rate
limiting / anti-tamper on inbound region spam is network-security-auth's side.

Related: [[authorized-teleport-paths]] — a region-driven travel is a new row in
that table and inherits every hazard in it.
