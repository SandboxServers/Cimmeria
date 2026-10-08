---
name: Dakara DK-04 tent travel
description: World-gated four-flap route and evidence boundaries for the shared tent interior
type: project
---

2026-10-07, DK-04 implementation: the owner chose the map estimates in `docs/analysis/dakara-e1-rebuild/placements/` and one client-shipped interior (world 62) for two adjacent exterior tents. Flap seeds live in `db/resources/Worlds/Seed/spawnlist_dakara_e1.sql`; chains 8003-8006 in `db/resources/Content/Seed/dakara_e1_space_chains.sql` use distinct tags plus explicit world conditions. `fire_interact_tag` now calls `populate_world_context` (`crates/cell-content/src/cell/content/event_dispatch/interaction.rs`); the older warning in `harset_space_chains.sql` that it never does is stale. `cross_world_teleport` sends `GateTravel` and then destroys the cell entity (`executor/transport.rs`), so a transient source-flap counter cannot select the return route. Both exits use PL-DK-A-04's open court. DK-02 places respawner 25 on the interior floor. Automated guards are in `chain_replay_tests/dakara_e1_tent_travel.rs` and `spawner_tests/dakara_e1/travel.rs`; the client walk and relog remain pending as documented in `docs/analysis/dakara-e1-rebuild/worknotes/DK-04.md`.
