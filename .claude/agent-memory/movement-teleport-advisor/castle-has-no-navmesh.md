---
name: castle-has-no-navmesh
description: Castle (world 8) has no castle.nav — NPC pathing there falls back to an unvalidated 3D straight line, and the miss only logs at DEBUG
metadata:
  type: project
---

> **Status 2026-09-19 — headline superseded.** `data/spaces/castle.nav` now exists: it shipped in #709
> ("ship castle.nav and seed Castle (world 8) advisory"). Castle is no longer meshless. Everything
> below about the *mechanism* — `find_path` returning `None`, the raw straight-line fallback with
> interpolated Y, `is_position_valid` failing open — still applies to every world that has no `.nav`,
> and the file paths and line numbers are as of 2026-09-18. Re-check them before citing.

**`data/spaces/castle.nav` does not exist.** Confirmed from the shipped tree and from colo telemetry
at `2026-09-18T23:49:37.821Z`: `"No navmesh for space (optional)"`, `world = "Castle"`,
`path = "data/spaces/castle.nav"`, `space_id = 65537`, severity **DEBUG**.

What *does* load: `Castle_CellBlock` (1479 polys, loaded on demand at world entry — it is instanced),
`Harset` (19345), `Agnos` (4062). `data/spaces/` holds only `agnos.nav`, `castle_cellblock.nav`,
`harset.nav`, `harset_storagerm.nav`, `sgc_w1.nav`. Everything else — SandBox, Lucia, Omega_Site,
Tollana, Agnos_Library, Sewer_Falls, Harset_CmdCenter, Dakara_E1, Ihpet_Crater_*, Menfa_*,
Beta_Site_Evo_1 — is meshless. All shipped meshes carry `agent_height = 0.6`, `agent_radius = 0.6`.

**Why it matters:** `SpaceManager::find_path` (`cell/space_manager/spatial.rs:41-51`) returns `None`
on `space.navmesh.as_ref()?`, and the callers then push a **raw unvalidated waypoint**:

- `cell/service/npc_ai/follow.rs:99-111` — `find_path(...).unwrap_or_default()`; when
  `path.len() <= 1` it pushes `dest`, which `:88-98` computes by linear interpolation along the
  NPC→target vector **in all three axes including Y**. Follower walks through walls, and climbs
  diagonally through the air when the leader is on a higher floor.
- `cell/service/npc_ai/fight.rs:451-452` — `min_range_backup` pushes a raw `compute_backup_waypoint`.
- `cell/service/npc_ai/investigate.rs:132` — same shape.

This is the mechanism behind the 2026-09-18 Castle report (Dr. Zerutska follower "came thru walls and
floors", ~00:26-00:29 UTC), and it is invisible: `is_position_valid` **fails open** when
`space.navmesh` is `None` (`spatial.rs:54-67`), matching the same fail-open that bites gate arrivals
in [[arrival-coordinate-offnavmesh]].

**How to apply:**
- Promote the missing-navmesh load (`cell/space_manager/lifecycle.rs:36-39`) from DEBUG to **WARN**,
  once per space. A world whose NPCs path blind is not an "optional" condition.
- Log when the straight-line fallback actually fires, with the vertical delta — a large `dy` is the
  air-climb signature.
- Before blaming NPC kinematics for a through-geometry report, **check whether that world has a
  mesh at all**. The symptom is world-scoped, not code-scoped.
- `crates/navmesh-extractor/` exists; whether Castle was skipped deliberately (size? extractor
  failure?) is an open question for the owner and gates the real fix.

Stacks with [[npc-broadcast-facing-and-grounding]]: no mesh means no ground truth *and* the FullPos
wire variant means the client won't correct the resulting Y either.
