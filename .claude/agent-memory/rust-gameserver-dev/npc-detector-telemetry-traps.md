---
name: npc-detector-telemetry-traps
description: Traps hit while building the NA02 NPC AI detectors — cross-test static races, a coincidental cover-coverage pass on the old seed, the missing movement-type wire message, and where detector state must be released.
metadata:
  type: project
---

Four non-obvious facts from NA02 (NPC AI detectors, branch `npcai/na02-detectors`, 2026-09-25):

- **A process-wide `static` read by the AI dispatcher races across parallel tests.** NA00's `LAST_OUTCOME` (the per-tick `decision_outcome` slot) was a `static Mutex`; once more tests drove `npc_ai_tick` concurrently, `tick_row` failed intermittently. Fixed with `tokio::task_local!` + `LAST_OUTCOME.scope(..)` per NPC turn (`npc_ai::with_outcome_slot`), which also follows the future across worker threads in production. **How to apply:** never add another cross-call static to the AI path; scope it.
- **(Historical, fixed by NA21 #780.)** The pre-NA21 cover seed was prefab-local, and ~3,000 of its nodes still landed on Cellblock's `y ≈ 0.2` ground plane by coincidence, so an "any node on the mesh" test proved nothing. Coverage now checks the world-scoped index with `get_height_near` around each node's own Y.
- **There is no server-to-client movement-type message** (NA10 Ghidra): `broadcast_movement_type` sent a truncated `onSequence`; the client animates NPCs from velocity alone. So `animating_without_path` is `stale_velocity` with `path_state = empty`, and nothing should key on `last_movement_type` as "what the client shows".
- **Detector state lives in `SpaceManager::npc_detectors` and must be released in both `destroy_entity` and `destroy_space`.** `destroy_space` also never released `zero_health_npc_log` before NA02. The guard is `detector_state_is_released_on_destroy_entity_and_destroy_space`.

Related: [[ai-state-private-and-revert-proof-mtime]], [[tracing-span-fields-not-on-log-records]] (LogCapture records an `Option<T>` field as the bare inner value, and omits it when `None`).
