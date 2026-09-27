---
name: npc-ai-tick-snapshot-and-hash-order
description: npc_ai_tick snapshots every NPC's ai_state before the loop and visits NPCs in HashMap order; per-NPC hooks must re-read state, and multi-NPC fixture tests go order-dependent
metadata:
  type: project
---

`npc_ai::dispatch::npc_ai_tick` snapshots `(id, ai_state, ...)` for every NPC **before** the loop, then runs the handlers one NPC at a time. An NPC that runs earlier in the same tick can change a later NPC's state: a mob's hit preempts its target into Fighting through `generate_threat`, and a leash drains players.

**Why:** in PT-05 (pets), the pet pre-pass trusted the snapshot `Follow`, re-armed Follow on a pet that had just been preempted into Fighting, and left it with a live threat list.

**How to apply:**
- Any per-NPC hook in the tick that writes state (like `pet::pre_pass`) should re-read `e.ai_state()` from the entity, not use the snapshot.
- Fixture tests with two or more ticking NPCs are order-dependent. The visit order is `HashMap` order, which is random per process, so nextest (one process per test) exposes it and `cargo test` often does not.
- Run such a test 3 or more times under nextest. Then either make the other NPC non-ticking (Idle and not hostile, or out of range so it cannot hit), or accept both outcomes explicitly.
- A Fighting mob with an empty threat list leashes and drains every player that lists it, so it can clear state before the NPC the test is about gets its turn.

Related: [[npc-ai-fight-test-fixtures]], [[cargo-test-vs-nextest-flakiness]].
