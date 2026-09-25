# NPC AI / Spawn advisor — memory index

- [spawn-timing-instanced-spaces.md](spawn-timing-instanced-spaces.md) — instanced-space NPCs spawn at CreateEntity (before ConnectEntity); spawnlist is the ONLY spawn source — content chains never create entities
- [castle-cellblock-navmesh-components.md](castle-cellblock-navmesh-components.md) — castle_cellblock.nav component map is for the OLD 2013 mesh (#694 rebuilt it, 17 comps); Preparation room (comp 24) is NOT walkable to topside (comp 8); topside route is one component
- [npc-follow-state-gaps.md](npc-follow-state-gaps.md) — Follow after GC1b-0/PR #646: use_player + templates.move_speed + leash snap-skip all landed; Follow still never resumes after combat
- [hostility-and-stationary-gates.md](hostility-and-stationary-gates.md) — faction==10 is the only damageable gate (so most seeded NPCs are unkillable); aggression has no DB column; is_stationary only affects fight.rs
- [content-spawn-traps.md](content-spawn-traps.md) — faction 0 = auto-aggro dead zone (players are always faction 0); set_visible on an NPC is dropped by base; respawn tick keys on ai_state+respawn_at, NOT spawn_id
- [submit-state-semantics.md](submit-state-semantics.md) — AiState::Submit: enum recovered, behavior invented; 6 unconditional writers exit it; `aggression = 0` is the default, not a pacify marker
- [npc-death-credit-and-respawn-gaps.md](npc-death-credit-and-respawn-gaps.md) — respawn_secs was NULL across the whole seed before CA05/#667 (Castle World 8 hostiles now 120 s; other worlds still NULL) and DoT-pulse kills never fire the death path / entity_dead_tag
- [ai-telemetry-and-aggro-dead-ends.md](ai-telemetry-and-aggro-dead-ends.md) — decision_outcome split into log-only vs counter-only halves; no `aggression` column in seed so proximity/social aggro never fires; leash predicate measures spawn→target; follow.rs:99 swallows path failure
- [harset-zone-evidence.md](harset-zone-evidence.md) — worlds.flags does NOT drive instancing (spaces.xml does); harset.nav = 1939 components, 9/12 spawns off-mesh; respawn_secs/patrol/wander/ability_set are NULL table-wide; fight-state has no straight-line fallback
- [faction-10-gates-everything.md](faction-10-gates-everything.md) — faction==10 is the ONLY switch for player-can-damage AND right-click-attacks; no runtime set_faction exists, so talk-then-kill needs two templates
- [level-is-hp-and-xp.md](level-is-hp-and-xp.md) — template level only drives HP (200+50*lvl), XP (10*lvl) and onLevelUpdate; level 50 is the seed's unknown-level sentinel, not a boss tier
- [template-seed-column-traps.md](template-seed-column-traps.md) — only ability_sets 1/2/3 exist (FK); NULL static_mesh on a prop = invisible; class being/spawnable never AI-ticks; NPCs have infinite ammo
- [leash-and-fight-exit-traps.md](leash-and-fight-exit-traps.md) — fight->Idle/leash keep nav_path + player threat; Idle agg-0 never ticked; find_path 0.5 start box vs on_navmesh; partial paths silent
- [leash-reset-na12.md](leash-reset-na12.md) — NA12 leash: NPC->spawn metric, 5 u band, walk home + evade, 5 s re-aggro window; Instant-based clocks hide loops in tests
- [faction-derived-aggro-na13.md](faction-derived-aggro-na13.md) — NA13: faction 10 aggroes on sight (players react as faction 3); chain-armed spawns need aggression_override=3; wire is onAggressionOverrideUpdate
- [assist-aggro-na14.md](assist-aggro-na14.md) — NA14 assist hooks generate_threat; shot faction-10 NPCs now pull neighbours <10 u, so test bystanders need a NEUTRAL pin
- [cover-behaviour-na22.md](cover-behaviour-na22.md) — startup spawns precede cover/world ids (sweep in cover_loaded); guards authored at markers; LoS from a slot reads blocked
- [cover-peek-los-na23.md](cover-peek-los-na23.md) — NA23: NPC at a slot looks from a peek point past its prop; over-prop peeks not walk-checked; mess tables stay blind
