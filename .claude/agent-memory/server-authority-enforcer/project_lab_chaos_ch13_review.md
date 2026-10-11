---
name: project-lab-chaos-ch13-review
description: CH-13 review (2026-10-11) of .despawn <entityId> and .spawnrandom batches; cleared shape plus two Low residuals (cross-space oracle, recycled-slot summary)
metadata:
  type: project
---

CH-13 (commit e10b4e1f1): `.despawn <entityId>` skips the selection and its #844 in-view check via
`Target::MobOrId` (dispatch.rs `resolve_target` early return to `spawn::resolve_npc_id`).
`.spawnrandom` copies form a batch in `SpaceManager::resources` (spawn/batch.rs) and the lab tee
parks its oneshot on it.

Cleared: the GM gate is unchanged (chat gate and the lab_console access_level gate, both upstream of
`handle_console_command`). The audit info row and the Discord row still fire for id despawns, and the
subject is named. NPC ids come from the monotonic `next_npc_id` (starts at 100_000), so a stale id
cannot name a different NPC. The batch closes in at most about 6 s (5 s deadline plus the 1 Hz
tick), so the lab-mcp `exec_console` await, which has no timeout of its own, is bounded. The count
clamp of 1-50 is unchanged.

Residuals (Low, GM-only):
- `resolve_npc_id` checks player, pet and class before the same-space check, so a GM learns those
  facts for an id in any space, including other players' instances. `.info <id>` deliberately
  collapses a cross-space id to "Could not find entity".
- The deferred summary and the late "after its .spawnrandom report" line go to `batch.requester`, a
  bare entity id. Player slots are recycled, so if the GM leaves within the window, a GM console line
  can reach a non-GM. This follows [[exploit_entity_id_recycling]] and [[pattern_deferred_name_resolution]].

By design: a GM can remove any same-space mob or being (vendor, quest giver, boss mid-fight, another
GM's dummy) without seeing it, until the next restart. Instances have their own space ids, so a GM
cannot reach another player's instance this way.
