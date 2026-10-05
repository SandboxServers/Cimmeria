---
name: debug-area-dial-hub
description: DA-07 (2026-10-04) Debug Area gate 29 outbound-only dial hub — grant-not-bypass design, where the hub filter lives, measured gate arrival survey, and three seed/cooked traps found on the way
metadata:
  type: project
---

Gate 29 (`Debug Area`, world 1300) carries `stargates.debug_dial_hub = true`.
A GM opening its DHD gets every gate on an *enterable* world pushed into the
in-memory book (`cell-interactions/.../gate_travel/dial_hub.rs`, called from
`try_open_dhd` before `onDisplayDHD`); the dial is still judged only by
`address_book::player_knows_stargate`, which refuses hub targets before
reading the book. Hub filtered in: gmDHD grant, content grant, `persist_arrival`
(both halves), base `append_known_stargate`.

**Why:** owner wanted "outbound dial any gate"; one authorization surface was
the gate-travel.md rule, so the GM power is a grant (like gmDHD), never a
bypass parameter. Grant on DHD open, not world entry, because the base's
`setupStargateInfo` at map load would race/overwrite a cell push.

**How to apply / traps found (all verified 2026-10-04):**

- "Enterable" = `SpaceManager::world_is_enterable` (startup space or instanced).
  14 of 28 2009 gate worlds have no space; dialling one destroys the cell
  entity and then `find_or_create_space` errors. Plain `handle_dial_gate` still
  has no such guard for non-hub dials (left alone: test fixtures dial worlds
  that have no space).
- Strict-arrival survey (as if enforced, NA28 meshes): only gate 22
  Menfa_Light (13.2 m to mesh) and 27 SGC_W1 (3.4 m) are off-mesh with no
  respawner; every other loadable gate row is on-mesh. All are advisory, so
  production accepts them. Pinned in `gate_travel/tests/debug_area_live_db.rs`.
- Template 1 (`GLB-DHD_00`) shipped `interaction_type = 0`: every DHD but the
  Castle's (162) was unclickable despite H01 claiming otherwise. DA-07 set 16.
- Cooked `CookedDataStargates.pak` addresses differ from the seed for gates
  20, 22, 24 (and light/dark pairs share glyphs in the PAK). The client
  resolves glyphs via the cooked table, so seed address columns are not what
  the client dials by. A new gate id needs a category-13 addition
  (`resources/src/base/stargate_overrides.rs`).
- Gate 29's arrival pin (Z1, respawner 130) exists only for `.gotolocation`
  entry-point placement; gate rows used as entry points land inside their own
  REGION_FLAG_Stargate volume otherwise.

Related: [[arrival-coordinate-offnavmesh]], [[na26-all-worlds-navmesh]],
[[cross-world-teleport-arrival-path]].
