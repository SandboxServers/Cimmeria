---
name: lab-console-tee-misses-async-spawn-ids
description: server_console_exec never returns `.spawn`/`.spawnrandom` entity ids (they arrive after the reply); no console command despawns by id
metadata:
  type: project
---

`server_console_exec` (LabConsoleExec tee, `crates/cell/src/cell/service/base_messages/lab_console.rs`) captures only GM feedback emitted synchronously while the command runs. `.spawn` / `.spawnrandom` reply just "requested N x template T"; the real `spawned npc <id> (template T)` lines come later from `GmSpawnNpcReady` (`base_messages/gm_spawn.rs`) after the base round-trip, and go to the client's chat, not the lab reply. `.despawn` and native `gmDespawn` act on the caller's selected target only (`dispatch::resolve_target`), never an id argument.

**Why:** found 2026-10-11 when lab-chaos CH-08 (burst profile) needed "despawn exactly what the run spawned"; the packet stopped and went back to the coordinator.

**How to apply:** any lab automation that spawns and must clean up needs a new id source (an entity_query diff, or a lab-mcp despawn-by-id tool). Don't parse the console reply for ids, and don't retarget the player to despawn.
