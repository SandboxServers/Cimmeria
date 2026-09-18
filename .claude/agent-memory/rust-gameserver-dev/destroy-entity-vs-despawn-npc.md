---
name: destroy-entity-vs-despawn-npc
description: SpaceManager::destroy_entity (bare) leaves the target in every witness's `witnesses` set with no immediate LeftAoI — despawn_npc is the correct primitive for any content-authored NPC removal that must be visible to observers right away.
metadata:
  type: project
---

`SpaceManager::destroy_entity` (`crates/services/src/cell/space_manager/entities.rs`) is a pure state-removal primitive: it drops the entity from `space.entities` and the spatial grid, but does **not** touch any other entity's `witnesses` set and sends no `LeftAoI`. The next AoI tick will *eventually* notice and clean up, but only for players the tick happens to visit, and only after up to a full tick of the client rendering a corpse that's already gone server-side. This is the same failure shape as the Castle Cellblock invisible-corpse bug (issue #582).

`SpaceManager::despawn_npc` (same file) is the correct primitive for any NPC removal that content or a GM command triggers and that observers need to see disappear immediately: it fans `LeftAoI` to every witness synchronously, scrubs the target out of every other entity's `witnesses` set in the same pass, then calls `destroy_entity`. It is NPC-only — refuses a player target structurally (`DespawnOutcome::RefusedPlayer`), independent of any caller-side check. Already used by the `.despawn` GM console command (`cell/console/spawn/`).

**Found 2026-09-17 (C08b, Castle Cellblock rebuild):** `content::executor::world::destroy_tagged_entity` (the `Action::DestroyTaggedEntity` arm, used by content chains via the `destroy_entity` seed verb) was calling the bare `destroy_entity`, not `despawn_npc` — meaning every content-authored NPC despawn (e.g. chain 1032's `ArmYourself_AmbernolVial`, and now chain 1161's Col Marsh despawn) had this exact latent gap. Fixed by switching to `despawn_npc`; the function became `async` and gained a `tx: &mpsc::Sender<CellToBaseMsg>` parameter as a result. Revert-verified: reverting the fix drops the witness-fanout test's assertion from `[(1,101),(2,101)]` to `[]` with zero panics (silent, not a crash — exactly the kind of regression a resolve-only chain-replay test cannot catch, per TESTING.md's `move_entity` warning).

**Rule of thumb:** if a content chain, GM command, or any other caller destroys an NPC and observers (other players in AoI) might be watching, always reach for `despawn_npc`, never the bare `destroy_entity`. Grep for other `space_mgr.destroy_entity(` call sites when touching this area again — `deferred_content_actions.rs`'s teardown-on-disconnect use of the bare call is correct as-is (the disconnecting/destroyed entity itself has no witnesses left to notify by that point in its own teardown), but any *external* target destroy should be audited.
