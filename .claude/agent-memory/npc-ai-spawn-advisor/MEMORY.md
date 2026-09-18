# NPC AI / Spawn advisor — memory index

- [spawn-timing-instanced-spaces.md](spawn-timing-instanced-spaces.md) — instanced-space NPCs spawn at CreateEntity (before ConnectEntity); spawnlist is the ONLY spawn source — content chains never create entities
- [castle-cellblock-navmesh-components.md](castle-cellblock-navmesh-components.md) — castle_cellblock.nav = 50 disconnected islands; Preparation room (comp 24) is NOT walkable to topside (comp 8); topside route is one component
- [npc-follow-state-gaps.md](npc-follow-state-gaps.md) — AiState::Follow works but can't target a player, never resumes after combat (leash snaps to spawn), and move_speed is hardcoded 6.0 u/s vs player 8.125
- [hostility-and-stationary-gates.md](hostility-and-stationary-gates.md) — faction==10 is the only damageable gate (so most seeded NPCs are unkillable); aggression has no DB column; is_stationary only affects fight.rs
- [content-spawn-traps.md](content-spawn-traps.md) — faction 0 = auto-aggro dead zone (players are always faction 0); set_visible on an NPC is dropped by base; respawn tick keys on ai_state+respawn_at, NOT spawn_id
- [submit-state-semantics.md](submit-state-semantics.md) — AiState::Submit: enum recovered, behavior invented; 6 unconditional writers exit it; `aggression = 0` is the default, not a pacify marker
- [harset-zone-evidence.md](harset-zone-evidence.md) — worlds.flags does NOT drive instancing (spaces.xml does); harset.nav = 1939 components, 9/12 spawns off-mesh; respawn_secs/patrol/wander/ability_set are NULL table-wide; fight-state has no straight-line fallback
- [faction-10-gates-everything.md](faction-10-gates-everything.md) — faction==10 is the ONLY switch for player-can-damage AND right-click-attacks; no runtime set_faction exists, so talk-then-kill needs two templates
- [level-is-hp-and-xp.md](level-is-hp-and-xp.md) — template level only drives HP (200+50*lvl), XP (10*lvl) and onLevelUpdate; level 50 is the seed's unknown-level sentinel, not a boss tier
- [template-seed-column-traps.md](template-seed-column-traps.md) — only ability_sets 1/2/3 exist (FK); NULL static_mesh on a prop = invisible; class being/spawnable never AI-ticks; NPCs have infinite ammo
