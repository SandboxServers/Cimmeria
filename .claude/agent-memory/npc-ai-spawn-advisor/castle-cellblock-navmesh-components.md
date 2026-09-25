---
name: castle-cellblock-navmesh-components
description: castle_cellblock.nav splits into 50 disconnected components; Preparation room and topside escape route are NOT connected, so NPC follow cannot cross the ring
metadata:
  type: project
---

> **STALE 2026-09-24:** #694 (2026-09-19) replaced this mesh with a rebuild from the client maps — 3,039 verts / 1,658 polys / **17 components** (`data/spaces/README.md`). The poly/component table below is for the OLD 2013 mesh; re-measure before relying on it.

`data/spaces/castle_cellblock.nav` (2778 verts, 1479 polys, bmin `[-400,-500,-400]`,
bmax `[400,500.6,400]`, cs 0.3, ch 0.2) flood-fills into **50 disconnected
components**. Measured 2026-09-17 by parsing the XRC header + poly neighbor
array offline (verts at byte 60, polys at 60 + nverts*6).

Component map for the Escape route:

| Location | poly | component |
|---|---|---|
| Player start (char creation) | 306 | **4** |
| Ring 2 pad (-192.7, 55.3, -154.8) | 538 | **24** |
| `Preparation_ColMarsh` spawn (-191, 54.7, -138.6) | 639 | **24** |
| Ring 3 pad (-89.7, 45.2, -161.5) | 489 | **8** |
| `MessHall_Guard1` | 1271 | **8** |
| `Hallway01_Guard` … `Hallway05_Guard1` | 1352/1409/1334/1367/1453 | **8** |
| `Barracks_Guard1` | 990 | **8** |

**Why:** the ring transport exists precisely because the Preparation room and
the topside floor are not walkable-connected. Recast built them as separate
islands, which matches the level design.

**How to apply:** any NPC escort across the ring must be a *teleport*
(`move_waypoint` snaps position), never a `find_path`. Once topside, the whole
route Ring3 → Mess Hall → Hallway01-05 → Barracks is one connected component,
so navmesh follow across the topside floor is viable with no new engine work.
`SpaceManager::find_path` returning `None` does NOT stop the NPC — `npc_ai_follow`
falls back to `nav_path.push_back(dest)`, a straight line through walls. A
cross-component follow therefore fails silently and visibly.

Related: [[npc-follow-state-gaps]]
