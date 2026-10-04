---
name: rule6-name-pairing-sweep-techniques
description: How to sweep Rule 6 (ID+name) log pairing fast and cheaply; site listing trick, lazy field eval, base-side name sources, traps hit in NT-23
metadata:
  type: reference
---

Learned in the NT-23 sweep (2026-10-04, world/movement/AoI/travel).

- **Listing the exact unpaired sites.** `unpaired_id_report` only prints top-10 files per crate. To get every `file:line key` for your files: copy the baseline aside, set your files' counts to 0 (fix the `# total` line to the new sum or the guard panics first), run `cargo nextest run -p cimmeria-server unpaired_id_fields_only_shrink --no-capture` through the lane; a rising count prints its sites. Restore the baseline after.
- **tracing evaluates field expressions only when the callsite is enabled**, so `entity_name = space_mgr.entity_label(id)` inside a disabled `trace!`/`debug!` costs nothing. When one lookup feeds several fields (NPC `EntityNames`), wrap the line in `if tracing::enabled!(target: "...", Level::DEBUG) { let n = ...; debug!(...) }`. `LogCapture` (thread-local default) satisfies `enabled!`.
- **Base side has no NPC names** except the `NpcAoIData.name_id` an `EnteredAoI` carries (`book().text(name_id)`); leave/move/method lines name players only, via `session_identity::player_name_for_entity` (one map hop, no scan). Never call it while holding the `connected` lock (it locks both maps).
- **Base space -> world**: `space_registry::world_for_space(space_id)` (reverse scan of the SpaceData registry, interned). Space ids are monotonic, so a stale row never misnames.
- **Names after teardown**: snapshot `entity_names`/`player_identity` (or intern the world with `name_intern::intern_opt`) before `destroy_entity`, an instance can vanish with its last player.
- **Method/class names**: use `cimmeria_wire::names` (NT-30): `class_name(u8)`, `player_client_method`, `player_cell_method`, `client_method(class, idx)`, `SpaceManager::client_method_name`. Don't hand-roll tables.
- **Scripted edits**: worktree `.rs` files are CRLF on disk; normalise `\r\n` before matching, restore after (see [[python-write-mangles-utf8-and-crlf]]).
- `world = ...unwrap_or("unknown")` on `movement.validation` rows was a Rule 6 violation, but the reject row's `UNKNOWN_WORLD` doubles as the metric label value: leave that one (Rule 4).
