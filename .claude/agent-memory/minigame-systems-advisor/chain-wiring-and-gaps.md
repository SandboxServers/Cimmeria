---
name: chain-wiring-and-gaps
description: How a content chain launches a minigame (start_minigame params), the full cell->minigame->cell victory loop, and the two hardcoded params that limit difficulty
metadata:
  type: project
---

# Launching a minigame from a content chain + the result loop

## Seed shape (the whole surface)

```sql
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1060, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1061]}', 0, 0);
```

- `target_key` = the **game name string**, matched verbatim against
  `games/mod.rs:11` (`"Livewire"`, `"Hack"`, …). Case-sensitive.
- `params.on_victory_chains` = array of chain ids fired on victory.
- **There is no difficulty param.** Loader: `crates/content-engine/src/loader/action.rs:110-121`
  reads only those two fields.

Live precedents in `db/resources/Content/Seed/castle_cellblock_chains.sql`:
lines 319, 536, 930 (chains 1016, 1041, 1060). Paired victory chains 1017,
1042, 1061. Login-restore chains 1045/1046/1065 re-set the
`INT_MinigameLivewire` (256) bit after a server restart — copy that pattern or
the icon disappears on relog.

## The loop, end to end

1. `Action::StartMinigame` → `CellToBaseMsg::StartMinigame`
   (`crates/services/src/cell/content/executor/mod.rs:203-228`).
2. Base registers a 64-hex one-time ticket keyed by `entity_id` and pushes
   `onStartMinigame(URL)` where URL = `http://unused/{host}/{port}/{game}/{entityId}/{ticket}`
   (`crates/services/src/base/world_entry/cell_dispatch/minigame.rs:36-80`).
3. Client loads the SWF, connects TCP, speaks SFS 1.x:
   `<msg t='sys'><body action='login'><login z='Livewire'><nick>42</nick><pword>TICKET</pword></login></body></msg>`
   — `nick` is the entity id, `pword` is the ticket, `z` is the game name.
   Validated in `session.rs:99-121` (ticket match AND game-name match).
4. Game runs; `GameOutput::Victory` → `CellToBaseMsg::MinigameResult { result_code: 1 }`
   (`minigame/server.rs:22-61,255-269`).
5. Base pushes `onEndMinigame()` and forwards `BaseToCellMsg::MinigameResult`
   (`cell_dispatch/minigame.rs:92-124`).
6. Cell fires every `on_victory_chains` id **only when `result_code == 1`**
   (`crates/services/src/cell/service/base_messages/minigame.rs:23-32`).

Result codes (RE, `findings/minigame-architecture.md:59-67`): 1 Success,
2 Failure, 3 Interrupted, 4 Defeated. Only 1 fires chains.

## Two hardcoded params that cap difficulty

- `difficulty: 1` — `cell/content/executor/mod.rs:213`, marked
  `// TODO: parse from chain params when difficulty field is added`.
- `tech_competency: 1` — `base/world_entry/cell_dispatch/minigame.rs:43`,
  `// TODO: read from player entity`.

Livewire consumes BOTH in `games/livewire/setup.rs:138-166` (goal count, timer
base, obstacle count, move timer all scale off them). So **every Livewire in
the game today is the easiest board**, regardless of the mission's level. This
is the single Rust change that unblocks difficulty-tiered minigames for
high-level content.

## Interaction bit must match the game

`docs/content/interaction-flags.md:59-65`: Livewire 256, Activate 512,
Analyze 1024, Bypass 2048, Converse 4096. There is **no `INT_MinigameHack`** —
the `Hack` game name has no dedicated cursor bit, so a "Hack the terminal"
step should use `Livewire` (256) unless someone finds a client mapping.

See also [[game-implementation-status]], [[converse-minigame-evidence]].
