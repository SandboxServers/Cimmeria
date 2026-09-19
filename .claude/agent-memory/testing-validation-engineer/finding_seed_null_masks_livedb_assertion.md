---
name: finding-seed-null-masks-livedb-assertion
description: A live-DB assertion that an override forced a field to None/0 is theatre when the picked seed row already has NULL there — verified on Harset H03
metadata:
  type: feedback
---

A live-DB or chain-replay test that asserts "the code forced field X to `None`/0
**regardless of** the source row" is only a guard if the fixture *proves* the
source row carried a non-default value. If the test picks its seed row with
`ORDER BY id LIMIT 1` and that row happens to have `X IS NULL`, the assertion
passes with the override deleted.

**Why:** Confirmed by revert on Harset H03. `spawn_npc_from_template` forces
`record.respawn_secs = None`. Deleting that line failed both unit tests (whose
fixture template deliberately carries `Some(30)`) but the live-DB replay
`mission_accept_spawns_a_tagged_npc_that_entity_dead_tag_can_complete_on`
**still passed** — its `assert_eq!(npc.respawn_secs, None, "…regardless of the
template row")` was satisfied by the seed's NULL, not by the override.

**How to apply:** When reviewing any "forced to default" assertion against live
data, ask what the source row actually holds. Either (a) pick the fixture row
with a `WHERE X IS NOT NULL` filter and assert the precondition inline the way
the unit fixture does, or (b) delete the assertion and let the unit test own
that invariant — a passing-either-way line in a live-DB test reads as coverage
it does not provide. The same shape applies to any COALESCE/override/clamp
guarded only against seed data.

Related: [[workflow_revert_audit]], [[finding_livedb_self_skip_masks_revert_verify]]
