---
name: castle-has-no-navmesh
description: Method for "NPC walked through walls" reports - a world with no .nav makes NPC pathing fall back to an unvalidated straight line and is_position_valid fail open; check whether the world has a mesh first
metadata:
  type: project
---

Castle (world 8) was meshless when this was written (2026-09-18); `data/spaces/castle.nav` shipped later (#709), so Castle is no longer the example. The mechanism applies to every world with no `.nav`.

**Mechanism:** `SpaceManager::find_path` returns `None` when `space.navmesh` is `None`, and the NPC AI callers (`follow.rs`, `fight.rs` min-range backup, `investigate.rs`) then push a raw waypoint. In follow, that waypoint is a linear interpolation in all three axes including Y, so a follower walks through walls and climbs diagonally when the leader is on a higher floor. `is_position_valid` also fails open with no mesh. The "no navmesh for space (optional)" miss logs only at DEBUG.

**Having a mesh is not ground truth either:** `get_navmesh_height` returned the wrong storey on multi-level meshes until NA01 (PR #774). OnGround keeps the client's current height (see [[npc-broadcast-facing-and-grounding]]).

**How to apply:**
- Before blaming NPC kinematics for a through-geometry report, check whether that world has a `.nav` in `data/spaces/` at all. The symptom is world-scoped, not code-scoped.
- Log the straight-line fallback with its vertical delta: a large `dy` is the air-climb signature.
- A missing mesh deserves WARN once per space, not DEBUG.

See [[arrival-coordinate-offnavmesh]] for the same fail-open on gate arrivals.

INVALIDATED WHEN: `find_path` or `is_position_valid` stop failing open without a navmesh, or every shipped world has a `.nav`.
checked: 2026-11-01
