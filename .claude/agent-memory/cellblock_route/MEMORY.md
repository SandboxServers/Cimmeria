# Cellblock route findings

- 2026-10-13: `client_move_to` already consumes ordered `waypoints` through real W and bound turn-key input and reports `native_level: real_input` (crates/lab/src/supervisor/world/movement.rs). The missing input was a navigable corridor source.
- The shipped `castle_cellblock.nav` has separate islands. The 2026-10-13 UAT failure from Preparation near (-196,55,-150) toward Mess Hall Guard1 (-96.25,34.591,-91.59) crosses a ring transport boundary; a single walk cannot reach it. The new `server_nav_path` reports no replayable waypoints for that query, preserving transport provenance.
- `NavMesh::find_path` can return `Partial` with a nonempty path that stops short of the requested end. Lab route output deliberately exposes corners only for `PathStatus::Ok`; regression guarded in `crates/cell/src/cell/service/base_messages/tests/lab_query.rs`.
