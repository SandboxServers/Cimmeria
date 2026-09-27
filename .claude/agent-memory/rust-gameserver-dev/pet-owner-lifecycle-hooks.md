---
name: pet-owner-lifecycle-hooks
description: Pets PT-02 — every player travel path must call pets::on_owner_left / on_owner_teleported; a source-scan test enforces it; owner gets no LeftAoI on travel
metadata:
  type: project
---

Since PT-02 (2026-09-27, branch `pets/pt-02-lifecycle`), `crates/cell-world/src/cell/pets/owner_hooks.rs` holds the two owner hooks:

- `on_owner_left(owner, reason, path: OwnerPath, tx, mgr)` before any player `destroy_entity` (travel, base destroy, death).
- `on_owner_teleported(owner, path: OwnerPath, tx, mgr)` after any same-space player snap (queued behind `TeleportPlayer` / `ReanchorPlayer`; the same-world ring calls it at `Effect::ShowPlayer`, not at the teleport, because the owner is still hidden then).

**Why:** `destroy_entity` has no `tx`, so without the hook the pet only goes on the next 100 ms sweep, and an instanced space destroyed with the owner swallows the pet with no `LeftAoI`.

**How to apply:**

- A new cell code path that sends `CellToBaseMsg::GateTravel` or `TeleportPlayer` fails `every_owner_travel_site_calls_the_pet_hooks` (cell-world) until it calls the matching hook. The movement snap-back (`base_messages/movement.rs`) is the only exemption.
- `PetDespawnReason::owner_view_torn_down()` reasons skip the owner's own `LeftAoI` (its client gets `RESET_ENTITIES`, and a leave queued behind `GateTravel` would be deferred into the new world's view). Owner death still notifies the owner.
- Ownership in a hook is the pet's summoner identity (`PetRegistry::summoner_matches(pet, live)`), never the bare owner id: ids are reused, and a teleport must not pull an old pet after the id's new holder (`owner_teleport_never_pulls_a_pet_the_id_holder_did_not_summon`). Per-pet log rows name the summoner via `teardown::owner_identity(mgr, pet, Some(owner))`.
- Pet-world fixtures for any crate: `cimmeria_cell_world::test_fixtures::{watched_pet_world, drain_left_aoi_for, drain_entity_moved_for, assert_pet_fully_gone}`.

Related: [[destroy-entity-vs-despawn-npc]], [[ring-transport-fsm]], [[cross-world-transfer-flow]].
