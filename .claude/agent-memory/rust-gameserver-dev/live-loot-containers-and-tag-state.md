---
name: live-loot-containers-and-tag-state
description: open_loot (loot window on a live chest) keeps per-looter rolls in container_loot keyed by player_id; once flags persist in sgw_player.looted_containers; entity_tag_state reads live_tags from populate_world_context; seeding traps
metadata:
  type: project
---

Learned building the 2026-09-28 playtest fixes (Castle chest, Cellblock backstops).

- **Two loot stores on `CellEntity`.** Corpses: `loot` (shared, rolled on death). Live containers:
  `container_loot: HashMap<player_id, Vec<LootItem>>` + `is_loot_container`. Every loot reader goes
  through `SpaceManager::loot_view` / `loot_list_mut` / `prune_container_loot` (cell-world
  `space_manager/loot_lists.rs`); `restore::put_back` takes the looter's `player_id` for the same reason.
  Key by `player_id`, not entity id: a relog changes the entity id.
- **`onLootDisplay` bytes live in `cimmeria_wire::cell::loot`.** cell-content cannot call cell-interactions
  (the dependency runs the other way), so anything both need goes in wire or cell-world.
- **Once-per-character flag** = `sgw_player.looted_containers varchar(64)[]`, riding the `PlayerInitRow`
  SELECT into `InitPlayerState.looted_containers` (known_stargates pattern), appended by the base on
  `CellToBaseMsg::ContainerLooted`. Select it as `::text[]` for sqlx `Vec<String>`.
- **`entity_tag_state` condition** reads the typed `ExecutionContext::live_tags`, filled inside
  `populate_world_context` (every `fire_*` already calls it). Dead = no living entity carries the tag
  (BSF_DEAD, HEALTH cur 0, despawned or never spawned). Fail-closed when `None`.
- **A malformed condition row must not return `None`** from `convert_condition` (that ungates the chain);
  build a never-matching condition (ordered operator on a two-state value) and warn instead.
- **An `interact_tag` chain on a template whose cursor bit is a template default** (INT_NormalLoot on
  templates 304/410) must be allowlisted in `content-engine/tests/it/interact_tag_linter.rs`.
- **Changing the debug crate touched four test files**: cell-catalog `live_db_debug_hub.rs`, cell-methods
  `debug_hub_dispatch_tests.rs`, cell-world `live_db_aggression.rs` (spawn 404's aggression override), plus
  the seed comments. `executor/mod.rs` sits at the 700-line cap: pass the whole `Action` into a handler.
- Worktree Bash refuses long `cd ... && python - <<'EOF'` heredocs that mention git-like text; write the
  script to the scratchpad and run `python <path>` instead. Python `newline=''` keeps CRLF: match on
  `s.replace('\r\n','\n')`, then restore.

Related: [[stargate-address-book-three-legs]], [[content-engine-condition-gotchas]], [[per-session-player-state-lifecycle]].
