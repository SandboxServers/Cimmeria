# AoI Witness Broadcast Agent — Memory Index

- [project_278_combat_death_fanout.md](project_278_combat_death_fanout.md) — #278 combat+death witness-fanout implementation: emit paths converted, idbase fix, tests added
- [reference_witness_fanout_helper.md](reference_witness_fanout_helper.md) — Location and signature of the three fanout helpers in messaging.rs; entity_is_player idbase threading
- [reference_despawn_npc_vs_destroy_entity.md](reference_despawn_npc_vs_destroy_entity.md) — despawn_npc vs bare destroy_entity semantics; death state is entity-local (no external table); looting_entity NOT cleared by despawn (gap)
- [reference_content_executor_catchall.md](reference_content_executor_catchall.md) — content/executor's Action match has a silent catch-all; SpawnEntity/DespawnEntity variants exist in the enum but have zero executor wiring as of H03
- [reference_periodic_aoi_tick_covers_midsession_spawn.md](reference_periodic_aoi_tick_covers_midsession_spawn.md) — unconditional 100ms AoI tick already introduces mid-session-spawned NPCs to connected witnesses; no explicit push needed; distinct from the #582 connect-race fix
- [project_na34_player_visibility_investigation.md](project_na34_player_visibility_investigation.md) — NA34 (2026-09-25): SigNoz found no post-#737 failure; added two-order regression test + aoi.introduce telemetry, root cause unconfirmed
