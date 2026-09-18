---
name: harset-zone-evidence
description: Harset worlds 57/68/69/70 — flags!=instanced, 23 spawnlist rows, harset.nav is 1939 components with 9/12 spawns off-mesh, no respawn/patrol/wander data anywhere
metadata:
  type: project
---

# Harset restoration evidence (measured 2026-09-17)

## `worlds.flags` does NOT drive instancing in Cimmeria

Instancing comes from `entities/spaces.xml` `Instanced="..."`
(`crates/services/src/cell/space_manager/xml.rs:56`, `xml.rs:87-97`).
`worlds.flags` is read nowhere in Rust. They disagree for Harset_CmdCenter:

| world | DB `flags` | spaces.xml `Instanced` | cell_spaces.xml startup |
|---|---|---|---|
| 57 Harset | 0 | false | yes |
| 68 Harset_CmdCenter | **1** | **false** | **yes** |
| 69 Harset_Market | 1 | true | no |
| 70 Harset_StorageRm | 1 | true | no |

Harset_Market's AABB in spaces.xml is degenerate (`MinX=MaxX=MinY=MaxY=0`).

## Table-wide seed gaps (not Harset-specific)

`db/resources/Entities/Seed/entity_templates.sql` has 153 rows but the INSERT
column list omits `respawn_secs`, `wander_radius`, `wander_*_dwell_secs`,
`follow_min/max_distance` entirely — all NULL for every template. Same for
`spawnlist.sql` (only spawn 10 sets `is_stationary`). Consequences:

- **Every NPC in the game is one-shot** — `respawn_secs` NULL on both sides
  means `respawn_at` is never stamped (`cell/combat/state.rs:110-112`).
- `patrol_path_id` non-NULL on **0/153** templates and 0/167 spawnlist rows.
- `ability_set_id` non-NULL on only templates **4, 15, 24**; `loot_table_id`
  only **15, 24**; vendor/trainer lists only on template **25** (debug NPC).
  So no Harset NPC has abilities or loot; NPCs fall back to
  `NPC_DEFAULT_ABILITY = 592` (`cell/combat/threat/aggro.rs:19`).
- `spawn_points` and `spawn_sets` seed files have **0 rows** — the
  SpawnRegion/SpawnSet system has no data at all; flat `spawnlist` is
  the only spawn source.

## harset.nav is effectively unusable

2,502,313 bytes, 39,652 verts / 19,345 polys, cs 0.30 ch 0.20,
bmin (-484.769,-277.999,-412.442) bmax (500,524.427,465.372).
Flood-fill → **1,939 connected components**, largest only 3,088 polys (16%),
671 singletons, 52.7% of edges have no neighbour.

Only 3 of 12 checked Harset spawns are on-mesh, and the DHD sits on a
**1-poly island**. `HarsetRingLeftBottom` + one Praxis Jaffa Guard are the
only two in a usable component (187, 1,316 polys). Both Lieutenants, Petbe,
the Merchant Basket and 4 of 5 ring switches are off-mesh.
`harset_storagerm.nav` = 1,917/963, 104 components.
`harset_cmdcenter.nav` and `harset_market.nav` do **not exist**.

Poly neighbour array stores the neighbour index **directly** (0-based),
`0xffff` = none, `0x8000|dir` = portal — NOT index+1. Verified 100%
reciprocal on harset.nav (35,560/35,560). Header layout confirmed at
`crates/entity/src/navigation/mod.rs:140-191`.

## No-navmesh behaviour (`crates/services/src/cell/service/npc_ai/`)

Missing .nav is swallowed to `None` at `space_manager/lifecycle.rs:35-39`
(debug log only). `find_path` → `None` (`space_manager/spatial.rs:49`).

- Follow / Patrol / Wander / Investigate **fall back to a straight line**
  (`follow.rs:99-110`, `patrol.rs:182-197`, `wander.rs:169-185`,
  `investigate.rs:121-133`).
- **Fight/chase has NO fallback** (`fight.rs:386-422`): `None` → logs
  `decision_outcome = "no_path"` and returns. The NPC aggros and then
  **stands still forever**.
- `has_line_of_sight` and `is_position_valid` **fail open** with no mesh
  (`spatial.rs:26`, `:63-66`) — so players move freely in CmdCenter/Market,
  and NPCs there have unconditional LoS.
- Spawner does **zero** navmesh validation (no snap, no drop).

## Mission 742 "Giving The Walls Ears" — the one scripted Harset mission

`deprecated/python/cell/missions/Harset/GivingTheWallsEars.py`. Binds dialogs
by **template id**, never spawns: template 163 (Petbe) → tags
`FirstBug`/`SecondBug`/`ThirdBug` (objectives 2913/2914/2915) → template 43
(Anat) → template 53 (Nerus). Blockers: only `FirstBug` exists in spawnlist
(spawn 224); `SecondBug`/`ThirdBug` appear nowhere in `db/` or `entities/`;
template 53 (Nerus) has **no spawn row in any world**.

`spaces/Harset.py` and `Harset_CmdCenter.py` only wire 5 ring switches and the
two transition regions (`moveTo` to `0,0.355,-20` / `0,-67.600,-231`) — they
create no entities, same as the Castle_CellBlock finding.

Related: [[spawn-timing-instanced-spaces]], [[npc-follow-state-gaps]],
[[castle-cellblock-navmesh-components]]
