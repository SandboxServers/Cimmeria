# NPC AI / Spawn advisor — memory index

- [spawn-timing-instanced-spaces.md](spawn-timing-instanced-spaces.md) — instanced-space NPCs spawn at CreateEntity (before ConnectEntity); spawnlist is the ONLY spawn source — content chains never create entities
- [castle-cellblock-navmesh-components.md](castle-cellblock-navmesh-components.md) — castle_cellblock.nav = 50 disconnected islands; Preparation room (comp 24) is NOT walkable to topside (comp 8); topside route is one component
- [npc-follow-state-gaps.md](npc-follow-state-gaps.md) — AiState::Follow works but can't target a player, never resumes after combat (leash snaps to spawn), and move_speed is hardcoded 6.0 u/s vs player 8.125
- [submit-state-semantics.md](submit-state-semantics.md) — AiState::Submit: enum recovered, behavior invented; 6 unconditional writers exit it; `aggression = 0` is the default, not a pacify marker
- [harset-zone-evidence.md](harset-zone-evidence.md) — worlds.flags does NOT drive instancing (spaces.xml does); harset.nav = 1939 components, 9/12 spawns off-mesh; respawn_secs/patrol/wander/ability_set are NULL table-wide; fight-state has no straight-line fallback
