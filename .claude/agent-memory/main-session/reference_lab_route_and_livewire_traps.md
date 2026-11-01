---
name: Lab route and Livewire atlas traps
description: Navmesh route output rules for lab queries and how the Livewire hit atlas is built and held; verify in the cited tests
metadata:
  type: reference
---

A lead, not evidence (2026-10-13 work).

## Routes

- `client_move_to` consumes ordered `waypoints` through real input (`crates/lab/src/supervisor/world/movement.rs`). A failing route is usually a missing corridor, not missing input.
- The shipped cellblock navmesh has separate islands joined by ring transports; a single walk cannot cross them, and `server_nav_path` reports no replayable waypoints there.
- `NavMesh::find_path` can return `Partial` with a nonempty path that stops short. Route output exposes corners only for `PathStatus::Ok`, with `reason: no_path` for partial and `reason: end_snap_far` when the end snaps over 1.5 m. Guard: `crates/cell/src/cell/service/base_messages/tests/lab_query.rs`. `client_move_to` uses a separate 0.5 m intermediate-waypoint radius.

## Livewire atlas

- `Livewire.upk` has one `GFxMovieInfo` export holding a GFX blob; `tools/upk_parser.py` resolves it. The generated atlas JSON is private to the lab install and never committed.
- `docs/analysis/cellblock-autoplay/livewire-spike/test_atlas.py` holds a golden per library; set `CIMMERIA_LIVEWIRE_UPK` to run it against a local client copy.
- The lab server clones per request, so keep the atlas as an `Arc` field on `LabServer`, not a local in router setup.
- `CIMMERIA_LAB_HOME` or the installer `-LabHome` relocates the lab cache.

INVALIDATED WHEN: lab route output or the Livewire solver is reworked.
checked: unverified 2026-11-01
