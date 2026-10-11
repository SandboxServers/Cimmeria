---
name: lab-console-tee-deferred-reply
description: The lab console tee captures only lines emitted while a command runs; a command whose result arrives via a base round trip must park the reply (CH-13 spawn batches); GmSpawnNpc carries a batch id
metadata:
  type: project
---

`server_console_exec` runs a `.`-line through the cell's lab tee
(`cell/service/base_messages/lab_console.rs`), which drains a private channel
right after `handle_console_command` returns. Anything produced later (a base
round trip such as `GmSpawnNpc` -> `GmSpawnNpcReady`) never reaches the reply.

CH-13 (2026-10-11) solved it for `.spawnrandom` without blocking the cell:
`console::spawn_batch::SpawnBatches` lives in `SpaceManager::resources`; the
tee takes `next_id()` before the command and `attach_lab(entity, mark,
LabWaiter)` after, so the reply is sent when the batch closes (all copies
answered, or 5 s timeout via the 1 Hz `spawn_batch_tick`). `GmSpawnNpc` /
`GmSpawnNpcReady` carry `batch: Option<u32>`; the base answers a batched
failure with `BaseToCellMsg::GmSpawnNpcFailed` instead of a chat line.

**Why:** the lab runner (CH-08 burst) needs spawned ids to despawn exactly
what it spawned; `.despawn <entityId>` (Target::MobOrId) is the other half.

**How to apply:** any new console command whose outcome is only known after
a base reply should reuse this park-on-a-table pattern (a mark before, an
attach after) rather than awaiting inside the command. Worknote:
`docs/analysis/lab-chaos/worknotes/CH-13.md`. `BaseToCellMsg` has no Debug
impl, so tests cannot `{:?}` it. See [[lab-mcp-raw-args-and-live-db-crate]].
