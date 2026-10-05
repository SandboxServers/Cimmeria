---
name: reference-uat-spec-authoring
description: Traps when writing a lab UAT spec (docs/guides/uat-specs/*.toml) - which tools plan as ready, typable chat, tiers, SigNoz fields that identify a spawn
metadata:
  type: reference
---

Learned writing `debug-area.toml` (DA-05, 2026-10-04). Verify against `crates/lab/src/uat/` before relying on it.

- **"Ready" means the plan test's `MAIN_TOOLS` list** in `crates/lab/src/uat/runner/tests.rs`, not what the live lab routes. `client_inventory`, `client_window_read`, `client_window_click`, `client_item_action` and `client_chat_log` are NOT in it, so a row whose step or *required* clause names one plans BLOCKED. Non-required clauses are not checked. Server and SigNoz clauses never block.
- **The plan test pins whole sections**: abilities and debug-area assert every row without `blocked` plans SKIPPED. A new spec should add the same loop, so a row that picks up an unrouted tool fails CI.
- **Chat lines must be typable**: letters, digits, space and `- _ / .` only (`checks.rs::typable`). No commas, quotes or apostrophes in a `chat` action; names with `'` go in tool args.
- **Tiers**: a GM line in `step` costs the row its N1 PASS. Put repositioning in setup, walk between neighbours with `@move_to`, and set `required_native = "G"` only when the GM action is the point (e.g. `/gmkilltarget ${id}` with the id captured by an `@entity_find` clause `at` an earlier step).
- **SigNoz identification**: `npc_ai.aggro event=acquired` carries the spawn `tag`, `cause`, `target_name`, `target_tag`, `has_los`; `npc_ai.leash` carries `tag`; `npc_respawn_recreate` (target `spawner.npc_respawn`) has only `template_id` and `respawn_secs`, no tag. `character_created` has no explicit target (module path), filter by `event`.
- **Interact range is 5 u** (`interact_range.rs`): click rows must stand within it; `@world_click {name}` picks the nearest substring match.

Related: [[reference-campaign-closeout-status-docs]].
