---
name: entity-recreate-and-tautological-expectations
description: Rebuilding a client pawn (NPC respawn, player reanchor) needs a re-create, not deltas; SpaceManager::introduction_events is the AoI-enter intro; a byte test whose expectation goes through the handler's own builder cannot catch a builder bug
metadata:
  type: project
---

Learned 2026-09-28 fixing respawn-pose / dead-hotbar-after-respawn.

- A client pawn in its death pose (after `onSequence` Entity_Death) is not revived by
  `onStateFieldUpdate(0)` / `onStatUpdate` / `InteractionType`. Only a rebuilt pawn stands up.
  Players: the reanchor (`CREATE_BASE_PLAYER`). NPCs: `LeftAoI` then
  `SpaceManager::introduction_events(witness, npc)` (the exact AoI-enter list, shared with
  `compute_player_aoi`). Leave-then-enter, not a bare second create: the client's
  `EntityManager_EnterAoI` asserts `getEnterCount() > 0` for an entity it already holds.
- After a player pawn recreate, anything sent BEFORE the reanchor lands on the pawn the client
  destroys. Replay after it (`respawn::resync::resync_after_pawn_recreate`).
- Tautology trap: `reanchor_keeps_the_gm_class...` first built its expected bytes through
  `build_reanchor_packets` -> `build_reanchor_burst_body`, the function under test, and passed
  on reverted code. Hand-build the load-bearing bytes in the expectation. Always run the
  revert proof; it is what caught this.
- Bumping what a respawn sends can deadlock tests that hand `handle_respawn` a small bounded
  `mpsc::channel(16)` nobody drains; size them for the whole burst (64).

Related: [[revert-proof-commit-first]], [[witness-entity-method-dual-fn]].
