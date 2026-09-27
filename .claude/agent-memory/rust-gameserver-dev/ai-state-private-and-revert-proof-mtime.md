---
name: ai-state-private-and-revert-proof-mtime
description: Since NA00 `CellEntity::ai_state` is private (write only via npc_ai::set_ai_state); and restoring a file from a backup with copy2 keeps its old mtime so cargo silently reuses the mutated build during revert-proof checks
metadata:
  type: project
---

`CellEntity::ai_state` is a private field since NA00 (2026-09-25, branch `npcai/na00-telemetry-plumbing`). Read with `.ai_state()`; write only through `crate::cell::service::npc_ai::{set_ai_state, set_ai_state_on}` with an `AiTransitionReason`. Test fixtures use the `#[cfg(test)]` `npc_ai::force_ai_state(npc, state)`. The raw `replace_ai_state_unlogged` is guarded by a workspace-scan test in `npc_ai/transition.rs`. `combat::generate_threat` takes an `AggroCause` (Proximity / Damage / ContentThreat); `mark_npc_dead` takes a `world: &str`.

**Why:** audit gap T8 — every state change must emit `npc_ai.transition` so SigNoz has a per-NPC timeline.

**How to apply:** a new state change needs a new `AiTransitionReason` variant (enumerated, it is a metric label), not a raw write. Resolve `world_label(space_mgr, id)` BEFORE `get_entity_mut`.

Revert-proof trap: mutating sources, running tests, then restoring from a backup with `shutil.copy2` (or `cp -p`) restores the OLD mtime, which is older than the mutated build's fingerprint, so cargo reuses the mutated binary and the "clean" rerun fails for no reason. `touch` the restored files (or restore with a plain write) before re-running.

Same trap with plain `cp f f.bak; <mutate>; mv f.bak f` (SS-M1, 2026-09-27): the backup's mtime is the backup time, older than the mutated build, so the LAST mutated crate stays stale. It surfaced only when a later multi-crate `cargo test` run failed on a guard whose fix was in the tree (`MAX_MAIL_RECIPIENTS` still 11 in the binary). A per-crate rerun can look fine because any later edit to the same crate rebuilds it. Put `touch "$f"` after every restore in a proof script.
